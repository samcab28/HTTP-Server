//! HTTP Server implementation
//!
//! Main server that listens for connections and dispatches requests.

use crate::config::ServerConfig;
use crate::error::{ServerError, ServerResult};
use crate::server::connection::Connection;
use crate::server::response::HttpResponse;
use crate::server::router::Router;
use crate::utils::logging::{logger, LogLevel};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use std::thread;

/// HTTP Server
pub struct HttpServer {
    config: ServerConfig,
    router: Router,
    running: Arc<AtomicBool>,
    connections_served: Arc<AtomicU64>,
    start_time: SystemTime,
}

impl HttpServer {
    /// Create a new HTTP server
    pub fn new(config: ServerConfig, router: Router) -> Self {
        Self {
            config,
            router,
            running: Arc::new(AtomicBool::new(false)),
            connections_served: Arc::new(AtomicU64::new(0)),
            start_time: SystemTime::now(),
        }
    }

    /// Start the server and listen for connections
    pub fn start(&mut self) -> ServerResult<()> {
        let listener = TcpListener::bind(self.config.bind_addr)
            .map_err(|e| ServerError::Config(format!("Failed to bind to {}: {}", self.config.bind_addr, e)))?;

        // Non-blocking accept so we can honor shutdown without a wake-up connection
        listener
            .set_nonblocking(true)
            .map_err(|e| ServerError::Config(format!("Failed to set nonblocking: {}", e)))?;

        self.running.store(true, Ordering::SeqCst);

        logger().info(&format!(
            "Server started on {}",
            self.config.bind_addr
        ));

        // Manual accept loop to handle non-blocking listener
        loop {
            if !self.running.load(Ordering::SeqCst) {
                break;
            }

            match listener.accept() {
                Ok((stream, _addr)) => {
                    // Clone data needed per-connection and handle in a new OS thread
                    let router = self.router.clone();
                    let config = self.config.clone();
                    let connections_served = self.connections_served.clone();

                    thread::spawn(move || {
                        // Handle connection lifecycle inside the thread
                        let result = (|| -> ServerResult<()> {
                            let mut connection = Connection::new(stream)?;

                            connection.set_read_timeout(config.timeouts.read_timeout)?;
                            connection.set_write_timeout(config.timeouts.write_timeout)?;

                            // Clone context to avoid lifetime conflict
                            let ctx = connection.context().clone();
                            logger().log_request(LogLevel::Info, &ctx, "Connection established");

                            let request = match connection.read_request() {
                                Ok(req) => req,
                                Err(e) => {
                                    logger().log_request(
                                        LogLevel::Warn,
                                        &ctx,
                                        &format!("Invalid request: {}", e),
                                    );
                                    let _ = connection.send_error(&e);
                                    return Ok(());
                                }
                            };

                            logger().log_request(
                                LogLevel::Info,
                                &ctx,
                                &format!("{} {}", request.method.as_str(), request.path),
                            );

                            let start = std::time::Instant::now();
                            let mut response = match router.handle(&request) {
                                Ok(resp) => resp.with_request_id(ctx.id()),
                                Err(e) => {
                                    logger().log_request(
                                        LogLevel::Warn,
                                        &ctx,
                                        &format!("Handler error: {}", e),
                                    );
                                    HttpResponse::from_error(&e).with_request_id(ctx.id())
                                }
                            };

                            // HEAD semantics: same headers, empty body
                            if matches!(request.method, crate::server::requests::HttpMethod::Head) {
                                response = response.into_head();
                            }

                            // Capture status before consuming response
                            let status = response.status_code();
                            connection.send_response(response)?;

                            // Record metrics per route with status buckets
                            let dur = start.elapsed();
                            crate::utils::metrics::metrics().lock().unwrap().record_status(&request.path, status, dur);

                            logger().log_request(LogLevel::Info, &ctx, "Response sent");

                            Ok(())
                        })();

                        if let Err(e) = result {
                            logger().log(LogLevel::Error, &format!("Connection error: {}", e));
                        }

                        connections_served.fetch_add(1, Ordering::SeqCst);
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No pending connections; avoid busy-spin
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    logger().log(
                        LogLevel::Error,
                        &format!("Failed to accept connection: {}", e),
                    );
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }

        Ok(())
    }

    // Per-connection handling is performed in the thread closure in start()


    /// Get server statistics
    pub fn stats(&self) -> ServerStats {
        let uptime = self.start_time
            .elapsed()
            .unwrap_or_default()
            .as_secs();

        ServerStats {
            uptime_seconds: uptime,
            connections_served: self.connections_served.load(Ordering::SeqCst),
            bind_address: self.config.bind_addr.to_string(),
        }
    }

    /// Stop the server
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        logger().info("Server stopped");
    }

    /// Expose a clone of the running flag for external shutdown coordination
    pub fn running_flag(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }
}

/// Server statistics
#[derive(Debug, Clone)]
pub struct ServerStats {
    pub uptime_seconds: u64,
    pub connections_served: u64,
    pub bind_address: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigBuilder;
    use crate::server::Router;

    #[test]
    fn test_server_creation() {
        let config = ConfigBuilder::new().port(9999).build().unwrap();
        let router = Router::new();
        let server = HttpServer::new(config, router);

        let stats = server.stats();
        assert_eq!(stats.connections_served, 0);
    }
}

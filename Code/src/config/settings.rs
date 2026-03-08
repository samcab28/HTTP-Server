//! Server configuration management
//!
//! Handles configuration from CLI arguments and environment variables.
//! Uses the builder pattern for flexible configuration construction.

use crate::error::{ServerError, ServerResult};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Socket address to bind to
    pub bind_addr: SocketAddr,

    /// Data directory for file operations
    pub data_dir: PathBuf,

    /// Worker configuration per command type
    pub workers: WorkerConfig,

    /// Queue configuration per command type
    pub queues: QueueConfig,

    /// Timeout configuration
    pub timeouts: TimeoutConfig,

    /// Maximum concurrent connections
    pub max_connections: usize,

    /// Enable request logging
    pub enable_logging: bool,
}

/// Worker pool configuration
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// Number of workers per command type
    workers_per_command: HashMap<String, usize>,

    /// Default number of workers
    default_workers: usize,
}

/// Queue configuration
#[derive(Debug, Clone)]
pub struct QueueConfig {
    /// Queue depth per command type
    depth_per_command: HashMap<String, usize>,

    /// Default queue depth
    default_depth: usize,
}

/// Timeout configuration
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Timeout for CPU-intensive operations
    pub cpu_timeout: Duration,

    /// Timeout for IO-intensive operations
    pub io_timeout: Duration,

    /// Connection read timeout
    pub read_timeout: Duration,

    /// Connection write timeout
    pub write_timeout: Duration,

    /// Optional per-route timeouts (overrides CPU/IO defaults when submitting tasks)
    per_route_ms: HashMap<String, u64>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
            data_dir: PathBuf::from("./data"),
            workers: WorkerConfig::default(),
            queues: QueueConfig::default(),
            timeouts: TimeoutConfig::default(),
            max_connections: 1000,
            enable_logging: true,
        }
    }
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            workers_per_command: HashMap::new(),
            default_workers: 4,
        }
    }
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            depth_per_command: HashMap::new(),
            default_depth: 100,
        }
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            cpu_timeout: Duration::from_secs(60),
            io_timeout: Duration::from_secs(120),
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(30),
            per_route_ms: HashMap::new(),
        }
    }
}

impl WorkerConfig {
    /// Get number of workers for a specific command
    pub fn get_workers(&self, command: &str) -> usize {
        self.workers_per_command
            .get(command)
            .copied()
            .unwrap_or(self.default_workers)
    }

    /// Set workers for a specific command
    pub fn set_workers(&mut self, command: impl Into<String>, count: usize) {
        self.workers_per_command.insert(command.into(), count);
    }

    /// Set default worker count
    pub fn set_default(&mut self, count: usize) {
        self.default_workers = count;
    }
}

impl QueueConfig {
    /// Get queue depth for a specific command
    pub fn get_depth(&self, command: &str) -> usize {
        self.depth_per_command
            .get(command)
            .copied()
            .unwrap_or(self.default_depth)
    }

    /// Set queue depth for a specific command
    pub fn set_depth(&mut self, command: impl Into<String>, depth: usize) {
        self.depth_per_command.insert(command.into(), depth);
    }

    /// Set default queue depth
    pub fn set_default(&mut self, depth: usize) {
        self.default_depth = depth;
    }
}

/// Builder for ServerConfig
pub struct ConfigBuilder {
    config: ServerConfig,
}

impl ConfigBuilder {
    /// Create a new config builder with defaults
    pub fn new() -> Self {
        Self {
            config: ServerConfig::default(),
        }
    }

    /// Set the bind address
    pub fn bind_addr(mut self, addr: SocketAddr) -> Self {
        self.config.bind_addr = addr;
        self
    }

    /// Set the port (keeping the current IP)
    pub fn port(mut self, port: u16) -> Self {
        self.config.bind_addr.set_port(port);
        self
    }

    /// Set the data directory
    pub fn data_dir(mut self, path: PathBuf) -> Self {
        self.config.data_dir = path;
        self
    }

    /// Set workers for a specific command
    pub fn workers_for(mut self, command: impl Into<String>, count: usize) -> Self {
        self.config.workers.set_workers(command, count);
        self
    }

    /// Set default worker count
    pub fn default_workers(mut self, count: usize) -> Self {
        self.config.workers.set_default(count);
        self
    }

    /// Set queue depth for a specific command
    pub fn queue_depth_for(mut self, command: impl Into<String>, depth: usize) -> Self {
        self.config.queues.set_depth(command, depth);
        self
    }

    /// Set default queue depth
    pub fn default_queue_depth(mut self, depth: usize) -> Self {
        self.config.queues.set_default(depth);
        self
    }

    /// Set CPU timeout
    pub fn cpu_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeouts.cpu_timeout = timeout;
        self
    }

    /// Set IO timeout
    pub fn io_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeouts.io_timeout = timeout;
        self
    }

    /// Set per-route timeout override (milliseconds)
    pub fn timeout_for(mut self, route: impl Into<String>, timeout: u64) -> Self {
        let route = route.into();
        let key = if route.starts_with('/') { route } else { format!("/{}", route) };
        self.config.timeouts.per_route_ms.insert(key, timeout);
        self
    }

    /// Set max concurrent connections
    pub fn max_connections(mut self, max: usize) -> Self {
        self.config.max_connections = max;
        self
    }

    /// Enable or disable logging
    pub fn logging(mut self, enable: bool) -> Self {
        self.config.enable_logging = enable;
        self
    }

    /// Build the configuration
    pub fn build(self) -> ServerResult<ServerConfig> {
        // Validate configuration
        if self.config.bind_addr.port() == 0 {
            return Err(ServerError::Config("Port cannot be 0".to_string()));
        }

        if self.config.workers.default_workers == 0 {
            return Err(ServerError::Config(
                "Worker count must be at least 1".to_string(),
            ));
        }

        if self.config.queues.default_depth == 0 {
            return Err(ServerError::Config(
                "Queue depth must be at least 1".to_string(),
            ));
        }

        Ok(self.config)
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeoutConfig {
    /// Get a timeout for a specific route if configured
    pub fn get_for_route(&self, route: &str) -> Option<Duration> {
        self.per_route_ms.get(route).copied().map(Duration::from_millis)
    }

    /// Set a per-route timeout (milliseconds)
    pub fn set_for_route(&mut self, route: impl Into<String>, timeout_ms: u64) {
        self.per_route_ms.insert(route.into(), timeout_ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn per_route_timeout_lookup() {
        let mut t = TimeoutConfig::default();
        t.set_for_route("/grep", 1500);
        assert_eq!(t.get_for_route("/grep").unwrap().as_millis(), 1500);
        assert!(t.get_for_route("/other").is_none());
    }

    #[test]
    fn builder_sets_timeout_route() {
        let cfg = ConfigBuilder::new().timeout_for("/isprime", 2500).build().unwrap();
        assert_eq!(cfg.timeouts.get_for_route("/isprime").unwrap().as_millis(), 2500);
    }
}


//! Connection handling
//!
//! Manages individual TCP connections and request/response cycle.

use crate::error::{ServerError, ServerResult};
use crate::server::requests::HttpRequest;
use crate::server::response::HttpResponse;
use crate::utils::logging::RequestContext;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// Connection handler for a single TCP stream
pub struct Connection {
    stream: TcpStream,
    request_context: RequestContext,
}

impl Connection {
    /// Create a new connection from a TCP stream
    pub fn new(stream: TcpStream) -> ServerResult<Self> {
        // Get peer address for logging
        let peer_addr = stream
            .peer_addr()
            .ok()
            .map(|addr| addr.to_string());

        let mut request_context = RequestContext::new();
        if let Some(addr) = peer_addr {
            request_context = request_context.with_client(addr);
        }

        Ok(Self {
            stream,
            request_context,
        })
    }

    /// Set read timeout for the connection
    pub fn set_read_timeout(&mut self, timeout: Duration) -> ServerResult<()> {
        self.stream
            .set_read_timeout(Some(timeout))
            .map_err(ServerError::Io)
    }

    /// Set write timeout for the connection
    pub fn set_write_timeout(&mut self, timeout: Duration) -> ServerResult<()> {
        self.stream
            .set_write_timeout(Some(timeout))
            .map_err(ServerError::Io)
    }

    /// Get the request context
    pub fn context(&self) -> &RequestContext {
        &self.request_context
    }

    /// Read and parse the HTTP request
    pub fn read_request(&mut self) -> ServerResult<HttpRequest> {
        // Clone the stream for reading (TcpStream implements Clone via Arc internally)
        let stream_clone = self.stream.try_clone().map_err(ServerError::Io)?;

        HttpRequest::parse(stream_clone)
    }

    /// Send an HTTP response
    pub fn send_response(&mut self, response: HttpResponse) -> ServerResult<()> {
        let response_bytes = response.build();

        self.stream
            .write_all(&response_bytes)
            .map_err(ServerError::Io)?;

        self.stream.flush().map_err(ServerError::Io)?;

        Ok(())
    }

    /// Send an error response
    pub fn send_error(&mut self, error: &ServerError) -> ServerResult<()> {
        let response = HttpResponse::from_error(error)
            .with_request_id(self.request_context.id());

        self.send_response(response)
    }

    /// Shutdown the connection gracefully
    pub fn shutdown(&mut self) -> ServerResult<()> {
        use std::net::Shutdown;

        self.stream
            .shutdown(Shutdown::Both)
            .map_err(ServerError::Io)
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        // Attempt graceful shutdown, ignore errors
        let _ = self.shutdown();
    }
}

/// Helper to read limited data from a stream
pub fn read_with_limit<R: Read>(reader: &mut R, limit: usize) -> ServerResult<Vec<u8>> {
    let mut buffer = Vec::with_capacity(limit.min(8192));
    let mut chunk = vec![0u8; 4096];
    let mut total_read = 0;

    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break, // EOF
            Ok(n) => {
                total_read += n;

                if total_read > limit {
                    return Err(ServerError::InvalidHttp(
                        "Request too large".to_string(),
                    ));
                }

                buffer.extend_from_slice(&chunk[..n]);

                // Check for end of headers (double CRLF)
                if buffer.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(ServerError::Io(e)),
        }
    }

    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_read_with_limit_success() {
        let data = b"GET / HTTP/1.0\r\n\r\n";
        let mut cursor = Cursor::new(data);

        let result = read_with_limit(&mut cursor, 1024).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_read_with_limit_exceeded() {
        let data = vec![b'A'; 2000];
        let mut cursor = Cursor::new(data);

        let result = read_with_limit(&mut cursor, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_with_limit_empty() {
        let data = b"";
        let mut cursor = Cursor::new(data);

        let result = read_with_limit(&mut cursor, 1024).unwrap();
        assert!(result.is_empty());
    }
}

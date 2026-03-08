//! Logging and tracing utilities
//!
//! Provides request ID generation and logging infrastructure.
//! Request IDs enable distributed tracing across the system.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Global counter for request IDs
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Request context for tracing
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Unique request identifier
    pub request_id: String,

    /// Timestamp when request was received
    pub timestamp: u64,

    /// Client address (if available)
    pub client_addr: Option<String>,
}

impl RequestContext {
    /// Create a new request context with a unique ID
    pub fn new() -> Self {
        let count = REQUEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let request_id = format!("{:016x}-{:08x}", timestamp, count);

        Self {
            request_id,
            timestamp,
            client_addr: None,
        }
    }

    /// Create context with client address
    pub fn with_client(mut self, addr: impl Into<String>) -> Self {
        self.client_addr = Some(addr.into());
        self
    }

    /// Get request ID for headers
    pub fn id(&self) -> &str {
        &self.request_id
    }
}

impl Default for RequestContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

/// Simple logger implementation
pub struct Logger {
    enabled: bool,
    min_level: LogLevel,
}

impl Logger {
    /// Create a new logger
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            min_level: LogLevel::Info,
        }
    }

    /// Set minimum log level
    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.min_level = level;
        self
    }

    /// Log a message
    pub fn log(&self, level: LogLevel, message: &str) {
        if !self.enabled || level < self.min_level {
            return;
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        eprintln!("[{}] {} - {}", timestamp, level.as_str(), message);
    }

    /// Log with request context
    pub fn log_request(&self, level: LogLevel, ctx: &RequestContext, message: &str) {
        if !self.enabled || level < self.min_level {
            return;
        }

        let client = ctx
            .client_addr
            .as_deref()
            .unwrap_or("unknown");

        let full_message = format!("[{}] {} - {}", ctx.request_id, client, message);
        self.log(level, &full_message);
    }

    /// Debug log
    pub fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message);
    }

    /// Info log
    pub fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }

    /// Warning log
    pub fn warn(&self, message: &str) {
        self.log(LogLevel::Warn, message);
    }

    /// Error log
    pub fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new(true)
    }
}

/// Global logger instance (thread-safe)
static GLOBAL_LOGGER: OnceLock<Logger> = OnceLock::new();

/// Initialize global logger
pub fn init_logger(enabled: bool) {
    let _ = GLOBAL_LOGGER.set(Logger::new(enabled));
}

/// Get global logger reference
pub fn logger() -> &'static Logger {
    GLOBAL_LOGGER.get_or_init(|| Logger::new(true))
}

/// Log macro for convenience
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::utils::logging::logger().info(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::utils::logging::logger().error(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::utils::logging::logger().warn(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::utils::logging::logger().debug(&format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_context() {
        let ctx1 = RequestContext::new();
        let ctx2 = RequestContext::new();

        // Request IDs should be unique
        assert_ne!(ctx1.request_id, ctx2.request_id);

        // Should have timestamp
        assert!(ctx1.timestamp > 0);
    }

    #[test]
    fn test_request_context_with_client() {
        let ctx = RequestContext::new().with_client("127.0.0.1:8080");
        assert_eq!(ctx.client_addr.as_deref(), Some("127.0.0.1:8080"));
    }

    #[test]
    fn test_logger_levels() {
        let logger = Logger::new(true).with_level(LogLevel::Warn);

        // Below min level should not panic
        logger.info("This should be ignored");
        logger.warn("This should appear");
        logger.error("This should appear");
    }

    #[test]
    fn test_disabled_logger() {
        let logger = Logger::new(false);
        // Should not panic
        logger.error("This should be ignored");
    }

    #[test]
    fn test_log_ordering() {
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
    }
}

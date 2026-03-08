//! Error types for the HTTP server
//!
//! This module defines a unified error handling system using thiserror.
//! All errors in the application should be variants of ServerError.

use std::io;
use std::num::{ParseIntError, TryFromIntError};
use std::str::Utf8Error;

/// Main error type for the server
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// IO errors (file operations, network, etc.)
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// HTTP parsing errors
    #[error("Invalid HTTP request: {0}")]
    InvalidHttp(String),

    /// Invalid request method
    #[error("Method not allowed: {0}")]
    MethodNotAllowed(String),

    /// Route not found
    #[error("Route not found: {0}")]
    NotFound(String),

    /// Invalid parameters
    #[error("Invalid parameter '{param}': {reason}")]
    InvalidParameter { param: String, reason: String },

    /// Missing required parameter
    #[error("Missing required parameter: {0}")]
    MissingParameter(String),

    /// UTF-8 conversion error
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] Utf8Error),

    /// Integer parsing error
    #[error("Integer parsing error: {0}")]
    ParseInt(#[from] ParseIntError),

    /// Integer conversion error
    #[error("Integer conversion error: {0}")]
    TryFromInt(#[from] TryFromIntError),

    /// File operation error
    #[error("File operation failed: {0}")]
    FileOperation(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Internal server error
    #[error("Internal server error: {0}")]
    Internal(String),

    /// Timeout error
    #[error("Operation timed out")]
    Timeout,

    /// Resource exhausted (queue full, etc.)
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),
}

/// Result type alias for server operations
pub type ServerResult<T> = Result<T, ServerError>;

impl ServerError {
    /// Get HTTP status code for this error
    pub fn status_code(&self) -> u16 {
        match self {
            ServerError::NotFound(_) => 404,
            ServerError::MethodNotAllowed(_) => 405,
            ServerError::InvalidParameter { .. }
            | ServerError::MissingParameter(_)
            | ServerError::InvalidHttp(_)
            | ServerError::ParseInt(_)
            | ServerError::Utf8(_) => 400,
            ServerError::ResourceExhausted(_) => 503,
            ServerError::Timeout => 408,
            _ => 500,
        }
    }

    /// Get error message suitable for JSON response
    pub fn error_message(&self) -> String {
        self.to_string()
    }

    /// Create an invalid parameter error
    pub fn invalid_param(param: impl Into<String>, reason: impl Into<String>) -> Self {
        ServerError::InvalidParameter {
            param: param.into(),
            reason: reason.into(),
        }
    }

    /// Create a missing parameter error
    pub fn missing_param(param: impl Into<String>) -> Self {
        ServerError::MissingParameter(param.into())
    }

    /// Create a not found error
    pub fn not_found(path: impl Into<String>) -> Self {
        ServerError::NotFound(path.into())
    }

    /// Create an internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        ServerError::Internal(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_codes() {
        assert_eq!(ServerError::not_found("/test").status_code(), 404);
        assert_eq!(
            ServerError::invalid_param("num", "must be positive").status_code(),
            400
        );
        assert_eq!(
            ServerError::ResourceExhausted("queue full".to_string()).status_code(),
            503
        );
        assert_eq!(ServerError::internal("test").status_code(), 500);
    }

    #[test]
    fn test_error_messages() {
        let err = ServerError::invalid_param("num", "must be positive");
        assert!(err.error_message().contains("num"));
        assert!(err.error_message().contains("must be positive"));
    }

    #[test]
    fn test_helper_constructors() {
        let err = ServerError::missing_param("id");
        assert_eq!(err.status_code(), 400);

        let err = ServerError::not_found("/api/test");
        assert_eq!(err.status_code(), 404);
    }
}
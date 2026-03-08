//! HTTP/1.0 Server Library
//!
//! A concurrent HTTP server implementation following the HTTP/1.0 specification.

pub mod algorithms;
pub mod config;
pub mod error;
pub mod handlers;
pub mod io_operations;
pub mod jobs;
pub mod server;
pub mod utils;
pub mod workers;

// Re-export commonly used types
pub use config::{ConfigBuilder, ServerConfig};
pub use error::{ServerError, ServerResult};
pub use server::{HttpServer, Router};

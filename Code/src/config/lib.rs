//! Configuration module
//!
//! Re-exports configuration types and utilities.

pub use settings::{
    ConfigBuilder, QueueConfig, ServerConfig, TimeoutConfig, WorkerConfig, parse_arg,
};

pub mod settings;
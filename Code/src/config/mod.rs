//! Configuration module

pub mod settings;

// Re-export main types
pub use settings::{
    ConfigBuilder, QueueConfig, ServerConfig, TimeoutConfig, WorkerConfig,
};
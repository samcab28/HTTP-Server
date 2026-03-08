//! Request handlers module

pub mod basics;
pub mod handler_traits;

// CPU/IO intensive handlers (skeletons provided)
pub mod cpu_intensive;
pub mod io_intensive;
pub mod job_endpoints;
pub mod metrics;

// Re-export handler functions
pub use basics::{
    handle_createfile, handle_deletefile, handle_fibonacci, handle_hash,
    handle_help, handle_random, handle_reverse, handle_status, handle_timestamp,
    handle_toupper, handle_sleep, handle_simulate, handle_loadtest,
};

pub use handler_traits::QueryParamExt;

// Re-export CPU/IO handler functions for router wiring
pub use cpu_intensive::{
    handle_factor, handle_isprime, handle_mandelbrot, handle_matrixmul, handle_pi,
};
pub use io_intensive::{
    handle_compress, handle_grep, handle_hashfile, handle_sortfile, handle_wordcount,
};
pub use metrics::handle_metrics;

// Expose a global list of available routes for dynamic /help
use std::sync::OnceLock;
static ROUTES: OnceLock<Vec<String>> = OnceLock::new();

pub fn set_available_routes(routes: Vec<String>) {
    let _ = ROUTES.set(routes);
}

pub fn available_routes() -> &'static [String] {
    ROUTES.get().map(|v| v.as_slice()).unwrap_or(&[])
}

// Data directory configured at startup
static DATA_DIR: OnceLock<String> = OnceLock::new();

pub fn set_data_dir(path: String) {
    let _ = DATA_DIR.set(path);
}

pub fn data_dir() -> &'static str {
    DATA_DIR.get().map(|s| s.as_str()).unwrap_or("./data")
}
// Job endpoints
pub use job_endpoints::{
    handle_job_submit, handle_job_status, handle_job_result, handle_job_cancel, handle_job_list,
};

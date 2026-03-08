//! Basic endpoint handlers
//!
//! Implements simple endpoints like /timestamp, /reverse, /toupper, etc.

use crate::error::{ServerError, ServerResult};
use crate::handlers::handler_traits::QueryParamExt;
use crate::handlers::available_routes;
use crate::server::requests::HttpRequest;
use crate::server::response::{HttpResponse, JsonResponseBuilder};
use crate::utils::crypto;
use crate::workers::worker_manager::worker_manager;
use crate::workers::worker_types::WorkPriority;
use std::time::{SystemTime, UNIX_EPOCH};
use std::time::Duration;
use std::thread;

/// Handle /timestamp - Return current Unix timestamp
pub fn handle_timestamp(_req: &HttpRequest) -> ServerResult<HttpResponse> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let response = JsonResponseBuilder::new(200)
        .field_num("timestamp", timestamp)
        .field("unit", "seconds")
        .build();

    Ok(response)
}

/// Handle /reverse?text=... - Reverse a string
pub fn handle_reverse(req: &HttpRequest) -> ServerResult<HttpResponse> {
    let text = req.require_query_param("text")?;
    let reversed: String = text.chars().rev().collect();

    let response = JsonResponseBuilder::new(200)
        .field("original", text)
        .field("reversed", &reversed)
        .build();

    Ok(response)
}

/// Handle /toupper?text=... - Convert to uppercase
pub fn handle_toupper(req: &HttpRequest) -> ServerResult<HttpResponse> {
    let text = req.require_query_param("text")?;
    let upper = text.to_uppercase();

    let response = JsonResponseBuilder::new(200)
        .field("original", text)
        .field("upper", &upper)
        .build();

    Ok(response)
}

/// Handle /hash?text=... - Calculate SHA256 hash
pub fn handle_hash(req: &HttpRequest) -> ServerResult<HttpResponse> {
    let text = req.require_query_param("text")?;
    let hash = crypto::sha256_hex(text.as_bytes());

    let response = JsonResponseBuilder::new(200)
        .field("text", text)
        .field("sha256", &hash)
        .build();

    Ok(response)
}

/// Handle /random?count=N&min=A&max=B - Generate random numbers
pub fn handle_random(req: &HttpRequest) -> ServerResult<HttpResponse> {
    let count: usize = req.parse_param("count")?;
    let min: i64 = req.parse_param_or("min", 0)?;
    let max: i64 = req.parse_param_or("max", 100)?;

    if count > 10000 {
        return Err(ServerError::invalid_param(
            "count",
            "maximum is 10000",
        ));
    }

    if min >= max {
        return Err(ServerError::invalid_param(
            "min/max",
            "min must be less than max",
        ));
    }

    let numbers = crypto::generate_random_numbers(count, min, max);
    let numbers_json = format!(
        "[{}]",
        numbers
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );

    let response = JsonResponseBuilder::new(200)
        .field_num("count", count)
        .field_num("min", min)
        .field_num("max", max)
        .field_raw("numbers", numbers_json)
        .build();

    Ok(response)
}

/// Handle /help - List all available endpoints
pub fn handle_help(_req: &HttpRequest) -> ServerResult<HttpResponse> {
    let routes = available_routes();
    let endpoints_json = if routes.is_empty() {
        "[\"/help\",\"/status\"]".to_string()
    } else {
        format!(
            "[{}]",
            routes
                .iter()
                .map(|s| format!(r#""{}""#, s))
                .collect::<Vec<_>>()
                .join(",")
        )
    };

    let response = JsonResponseBuilder::new(200)
        .field("server", "HTTP/1.0 Server")
        .field_raw("endpoints", endpoints_json)
        .build();

    Ok(response)
}

/// Handle /fibonacci?num=N - Calculate Fibonacci number
pub fn handle_fibonacci(req: &HttpRequest) -> ServerResult<HttpResponse> {
    let n: u64 = req.parse_param("num")?;

    if n > 90 {
        return Err(ServerError::invalid_param(
            "num",
            "maximum is 90 (overflow protection)",
        ));
    }

    // Offload to CPU worker pool with timeout; fall back to error mapping
    let response = worker_manager().submit_cpu(move || {
        let start = SystemTime::now();
        let result = fibonacci(n);
        let elapsed = start.elapsed().unwrap_or_default().as_millis();

        JsonResponseBuilder::new(200)
            .field_num("n", n)
            .field_num("result", result)
            .field_num("elapsed_ms", elapsed)
            .build()
    })?;

    Ok(response)
}

/// Calculate Fibonacci number (iterative)
fn fibonacci(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    if n == 1 {
        return 1;
    }

    let mut a = 0u64;
    let mut b = 1u64;

    for _ in 2..=n {
        let temp = a.wrapping_add(b);
        a = b;
        b = temp;
    }

    b
}

/// Handle /status - Server status
pub fn handle_status(_req: &HttpRequest) -> ServerResult<HttpResponse> {
    let uptime = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let pid = std::process::id();

    let response = JsonResponseBuilder::new(200)
        .field_num("pid", pid)
        .field_num("uptime_seconds", uptime)
        .field("status", "running")
        .build();

    Ok(response)
}

/// Handle /createfile?name=...&content=...&repeat=N
pub fn handle_createfile(req: &HttpRequest) -> ServerResult<HttpResponse> {
    let name = req.require_query_param("name")?;
    let content = req.require_query_param("content")?;
    let repeat: usize = req.parse_param_or("repeat", 1)?;

    // Validate filename (no path traversal)
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err(ServerError::invalid_param(
            "name",
            "invalid filename",
        ));
    }

    if repeat > 100000 {
        return Err(ServerError::invalid_param(
            "repeat",
            "maximum is 100000",
        ));
    }

    let full_content = content.repeat(repeat);
    let file_path = format!("./data/{}", name);

    // Create data directory if it doesn't exist
    std::fs::create_dir_all("./data")
        .map_err(|e| ServerError::FileOperation(format!("Failed to create data directory: {}", e)))?;

    std::fs::write(&file_path, full_content.as_bytes())
        .map_err(|e| ServerError::FileOperation(format!("Failed to write file: {}", e)))?;

    let response = JsonResponseBuilder::new(200)
        .field("filename", name)
        .field_num("size_bytes", full_content.len())
        .field("status", "created")
        .build();

    Ok(response)
}

/// Handle /deletefile?name=...
pub fn handle_deletefile(req: &HttpRequest) -> ServerResult<HttpResponse> {
    let name = req.require_query_param("name")?;

    // Validate filename
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err(ServerError::invalid_param(
            "name",
            "invalid filename",
        ));
    }

    let file_path = format!("./data/{}", name);

    if !std::path::Path::new(&file_path).exists() {
        return Err(ServerError::not_found(format!("File not found: {}", name)));
    }

    std::fs::remove_file(&file_path)
        .map_err(|e| ServerError::FileOperation(format!("Failed to delete file: {}", e)))?;

    let response = JsonResponseBuilder::new(200)
        .field("filename", name)
        .field("status", "deleted")
        .build();

    Ok(response)
}

/// Handle /sleep?seconds=s
/// Offloads to IO pool; returns when the sleep finishes.
pub fn handle_sleep(req: &HttpRequest) -> ServerResult<HttpResponse> {
    let secs: u64 = req.parse_param("seconds")?;
    if secs > 3600 {
        return Err(ServerError::invalid_param("seconds", "maximum is 3600"));
    }
    let prio = req
        .query_params
        .get("prio")
        .map_or("normal", |s| s.as_str())
        .parse()
        .unwrap_or(WorkPriority::Normal);
    let timeout = worker_manager().io_timeout();
    let resp = worker_manager().submit_for_with_priority("/sleep", timeout, prio, move || {
        let start = SystemTime::now();
        thread::sleep(Duration::from_secs(secs));
        let elapsed = start.elapsed().unwrap_or_default().as_millis();
        JsonResponseBuilder::new(200)
            .field_num("seconds", secs)
            .field_num("elapsed_ms", elapsed)
            .build()
    })?;
    Ok(resp)
}

/// Handle /simulate?seconds=s&task=name
/// Simulates CPU work for s seconds by running computations in a loop.
pub fn handle_simulate(req: &HttpRequest) -> ServerResult<HttpResponse> {
    let secs: u64 = req.parse_param("seconds")?;
    if secs == 0 || secs > 3600 {
        return Err(ServerError::invalid_param("seconds", "1..=3600 allowed"));
    }
    let task: String = req.parse_param_or("task", String::from("cpu"))?;
    let prio = req
        .query_params
        .get("prio")
        .map_or("normal", |s| s.as_str())
        .parse()
        .unwrap_or(WorkPriority::Normal);
    let timeout = worker_manager().cpu_timeout();
    let resp = worker_manager().submit_for_with_priority("/simulate", timeout, prio, move || {
        let start = SystemTime::now();
        let deadline = start + Duration::from_secs(secs);
        let mut iterations: u64 = 0;
        // Different micro-tasks; default to CPU primality checks
        match task.as_str() {
            "hash" => {
                // Hash a growing buffer repeatedly
                let mut buf: Vec<u8> = vec![0u8; 1024];
                while SystemTime::now() < deadline {
                    for b in &mut buf { *b = b.wrapping_add(1); }
                    let _h = crate::utils::crypto::sha256_hex(&buf);
                    iterations += 1;
                }
            }
            _ => {
                // CPU: primality test on sequential numbers
                let mut n: u64 = 1_000_003; // a prime-ish start
                while SystemTime::now() < deadline {
                    let _ = crate::algorithms::prime::is_prime_trial(n);
                    n = n.wrapping_add(2);
                    iterations += 1;
                }
            }
        }
        let elapsed = start.elapsed().unwrap_or_default().as_millis();
        JsonResponseBuilder::new(200)
            .field("task", task)
            .field_num("seconds", secs)
            .field_num("iterations", iterations)
            .field_num("elapsed_ms", elapsed)
            .build()
    })?;
    Ok(resp)
}

/// Handle /loadtest?tasks=n&sleep=x
/// Submits n sleep tasks that each sleep x seconds, waits for all, and reports timing.
pub fn handle_loadtest(req: &HttpRequest) -> ServerResult<HttpResponse> {
    let tasks: usize = req.parse_param("tasks")?;
    let sleep_secs: u64 = req.parse_param("sleep")?;
    if tasks == 0 || tasks > 10000 { return Err(ServerError::invalid_param("tasks", "1..=10000")); }
    if sleep_secs > 3600 { return Err(ServerError::invalid_param("sleep", "maximum is 3600")); }
    let prio = req
        .query_params
        .get("prio")
        .map_or("normal", |s| s.as_str())
        .parse()
        .unwrap_or(WorkPriority::Normal);
    let io_timeout = worker_manager().io_timeout();

    // Offload orchestrator to avoid blocking the accept loop
    let resp = worker_manager().submit_for_with_priority("/loadtest", io_timeout, prio, move || {
        let start = SystemTime::now();
        // Spawn task-submitters which will wait on the worker pool
        let mut joiners = Vec::with_capacity(tasks);
        for _ in 0..tasks {
            let pr = prio; // copy
            let handle = std::thread::spawn(move || {
                // Each task sleeps on the IO route-specific pool
                let _ = worker_manager().submit_for_with_priority("/loadtest.worker", io_timeout, pr, move || {
                    thread::sleep(Duration::from_secs(sleep_secs));
                    0u8
                });
            });
            joiners.push(handle);
        }
        for h in joiners { let _ = h.join(); }
        let elapsed = start.elapsed().unwrap_or_default().as_millis();
        JsonResponseBuilder::new(200)
            .field_num("tasks", tasks as u64)
            .field_num("sleep_seconds", sleep_secs)
            .field_num("elapsed_ms", elapsed)
            .build()
    })?;
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fibonacci() {
        assert_eq!(fibonacci(0), 0);
        assert_eq!(fibonacci(1), 1);
        assert_eq!(fibonacci(10), 55);
        assert_eq!(fibonacci(20), 6765);
    }
}

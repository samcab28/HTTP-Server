//! IO-intensive endpoint handlers (skeleton)
//!
//! Parameter parsing + worker offload. Actual IO ops live in `io_operations`.

use crate::error::{ServerError, ServerResult};
use crate::handlers::handler_traits::QueryParamExt;
use crate::server::requests::HttpRequest;
use crate::server::response::{HttpResponse, JsonResponseBuilder};
use crate::workers::worker_manager::worker_manager;
use crate::workers::worker_types::WorkPriority;
use crate::utils::validation::validate_filename;
use crate::io_operations::{file_processing, hashing, file_ops, compression};
use crate::handlers::data_dir;
use std::time::SystemTime;
use crate::jobs::job_manager::job_manager;

// (removed unused not_implemented helper)

/// GET /sortfile?name=FILE&algo=merge|quick
pub fn handle_sortfile(req: &HttpRequest) -> ServerResult<HttpResponse> {
    if matches!(req.query_params.get("async").map(|v| v.as_str()), Some("1") | Some("true") ) {
        let mut params = req.query_params.clone(); params.remove("async");
        let id = job_manager().submit("/sortfile", params)?;
        return Ok(JsonResponseBuilder::new(200).field("job_id", id).field("route", "/sortfile").build());
    }
    let name = req.require_query_param("name")?;
    let algo: String = req.parse_param_or("algo", String::from("merge"))?;
    let name = name.to_string();
    validate_filename(&name)?;
    let path = format!("{}/{}", data_dir(), name);
    let algo_clone = algo.clone();
    let prio = req
        .query_params
        .get("prio")
        .map_or("normal", |s| s.as_str())
        .parse()
        .unwrap_or(WorkPriority::Normal);
    let timeout = worker_manager().io_timeout();
    let resp = worker_manager().submit_for_with_priority("/sortfile", timeout, prio, move || {
        let res = match algo_clone.as_str() {
            "merge" => file_ops::mergesort_file_external(&path),
            "quick" => file_ops::quicksort_file(&path),
            _ => return JsonResponseBuilder::new(400).field("error", "unsupported algo").build(),
        };
        match res {
            Ok((out_path, metrics)) => JsonResponseBuilder::new(200)
                .field("filename_in", &path)
                .field("filename_out", out_path)
                .field("algo", algo_clone)
                .field_num("lines", metrics.lines)
                .field_num("runs", metrics.runs as u64)
                .field_num("bytes_in", metrics.bytes_in)
                .field_num("bytes_out", metrics.bytes_out)
                .field_num("elapsed_ms", metrics.elapsed_ms)
                .build(),
            Err(e) => {
                let code = if e.kind() == std::io::ErrorKind::NotFound { 404 } else { 500 };
                JsonResponseBuilder::new(code)
                    .field("error", e.to_string())
                    .build()
            }
        }
    })?;
    Ok(resp)
}

/// GET /wordcount?name=FILE
pub fn handle_wordcount(req: &HttpRequest) -> ServerResult<HttpResponse> {
    if matches!(req.query_params.get("async").map(|v| v.as_str()), Some("1") | Some("true") ) {
        let mut params = req.query_params.clone(); params.remove("async");
        let id = job_manager().submit("/wordcount", params)?;
        return Ok(JsonResponseBuilder::new(200).field("job_id", id).field("route", "/wordcount").build());
    }
    let name = req.require_query_param("name")?.to_string();
    validate_filename(&name)?;
    let path = format!("{}/{}", data_dir(), name);
    let prio = req
        .query_params
        .get("prio")
        .map_or("normal", |s| s.as_str())
        .parse()
        .unwrap_or(WorkPriority::Normal);
    let timeout = worker_manager().io_timeout();
    let resp = worker_manager().submit_for_with_priority("/wordcount", timeout, prio, move || {
        let start = SystemTime::now();
        match file_processing::word_count(&path) {
            Ok(wc) => {
                let elapsed = start.elapsed().unwrap_or_default().as_millis();
                JsonResponseBuilder::new(200)
                    .field("filename", &path)
                    .field_num("lines", wc.lines)
                    .field_num("words", wc.words)
                    .field_num("bytes", wc.bytes)
                    .field_num("elapsed_ms", elapsed)
                    .build()
            }
            Err(e) => {
                let code = if e.kind() == std::io::ErrorKind::NotFound { 404 } else { 500 };
                JsonResponseBuilder::new(code)
                    .field("error", e.to_string())
                    .build()
            }
        }
    })?;
    Ok(resp)
}

/// GET /grep?name=FILE&pattern=REGEX
pub fn handle_grep(req: &HttpRequest) -> ServerResult<HttpResponse> {
    if matches!(req.query_params.get("async").map(|v| v.as_str()), Some("1") | Some("true") ) {
        let mut params = req.query_params.clone(); params.remove("async");
        let id = job_manager().submit("/grep", params)?;
        return Ok(JsonResponseBuilder::new(200).field("job_id", id).field("route", "/grep").build());
    }
    let name = req.require_query_param("name")?.to_string();
    validate_filename(&name)?;
    let pattern = req.require_query_param("pattern")?.to_string();
    if pattern.is_empty() { return Err(ServerError::invalid_param("pattern", "cannot be empty")); }
    let icase: u8 = req.parse_param_or("icase", 0)?;
    let overlap: u8 = req.parse_param_or("overlap", 0)?;
    let preview: usize = req.parse_param_or("preview", 10)?;
    let preview = preview.clamp(1, 100);
    let icase = icase != 0;
    let overlap = overlap != 0;
    let path = format!("{}/{}", data_dir(), name);
    let prio = req
        .query_params
        .get("prio")
        .map_or("normal", |s| s.as_str())
        .parse()
        .unwrap_or(WorkPriority::Normal);
    let timeout = worker_manager().io_timeout();
    let resp = worker_manager().submit_for_with_priority("/grep", timeout, prio, move || {
        let start = SystemTime::now();
        match file_processing::grep_file_opts(&path, &pattern, preview, icase, overlap) {
            Ok(res) => {
                let elapsed = start.elapsed().unwrap_or_default().as_millis();
                let first_json = format!(
                    "[{}]",
                    res.first_lines
                        .iter()
                        .map(|s| format!(r#""{}""#, s.replace('"', "\\\"")))
                        .collect::<Vec<_>>()
                        .join(",")
                );
                JsonResponseBuilder::new(200)
                    .field("filename", &path)
                    .field("pattern", &pattern)
                    .field_bool("icase", icase)
                    .field_bool("overlap", overlap)
                    .field_num("preview", preview)
                    .field_num("matches", res.matches)
                    .field_num("elapsed_ms", elapsed)
                    .field_raw("preview", first_json)
                    .build()
            }
            Err(e) => {
                let code = if e.kind() == std::io::ErrorKind::NotFound { 404 } else { 500 };
                JsonResponseBuilder::new(code)
                    .field("error", e.to_string())
                    .build()
            }
        }
    })?;
    Ok(resp)
}

/// GET /compress?name=FILE&codec=gzip|xz
pub fn handle_compress(req: &HttpRequest) -> ServerResult<HttpResponse> {
    if matches!(req.query_params.get("async").map(|v| v.as_str()), Some("1") | Some("true") ) {
        let mut params = req.query_params.clone(); params.remove("async");
        let id = job_manager().submit("/compress", params)?;
        return Ok(JsonResponseBuilder::new(200).field("job_id", id).field("route", "/compress").build());
    }
    let name = req.require_query_param("name")?.to_string();
    let codec: String = req.parse_param_or("codec", String::from("gzip"))?;
    let impl_flag: String = req.parse_param_or("impl", String::from("auto"))?;
    validate_filename(&name)?;
    let path = format!("{}/{}", data_dir(), name);
    let prio = req
        .query_params
        .get("prio")
        .map_or("normal", |s| s.as_str())
        .parse()
        .unwrap_or(WorkPriority::Normal);
    let timeout = worker_manager().io_timeout();
    let resp = worker_manager().submit_for_with_priority("/compress", timeout, prio, move || {
        let impl_hint = impl_flag.parse().unwrap_or(compression::ImplHint::Auto);
        let result = match codec.as_str() {
            "gzip" => compression::compress_gzip_select(&path, impl_hint),
            "xz" => compression::compress_xz_select(&path, impl_hint),
            _ => return JsonResponseBuilder::new(400).field("error", "unsupported codec").build(),
        };
        match result {
            Ok((out_path, m)) => JsonResponseBuilder::new(200)
                .field("filename_in", &path)
                .field("filename_out", out_path)
                .field("codec", codec)
                .field("impl", impl_flag)
                .field_num("bytes_in", m.bytes_in)
                .field_num("bytes_out", m.bytes_out)
                .field_num("elapsed_ms", m.elapsed_ms)
                .build(),
            Err(e) => {
                let code = if e.kind() == std::io::ErrorKind::NotFound { 404 } else { 501 };
                JsonResponseBuilder::new(code)
                    .field("error", e.to_string())
                    .build()
            }
        }
    })?;
    Ok(resp)
}

/// GET /hashfile?name=FILE&algo=sha256
pub fn handle_hashfile(req: &HttpRequest) -> ServerResult<HttpResponse> {
    if matches!(req.query_params.get("async").map(|v| v.as_str()), Some("1") | Some("true") ) {
        let mut params = req.query_params.clone(); params.remove("async");
        let id = job_manager().submit("/hashfile", params)?;
        return Ok(JsonResponseBuilder::new(200).field("job_id", id).field("route", "/hashfile").build());
    }
    let name = req.require_query_param("name")?.to_string();
    validate_filename(&name)?;
    let algo: String = req.parse_param_or("algo", String::from("sha256"))?;
    if algo.to_lowercase() != "sha256" {
        return Err(ServerError::invalid_param("algo", "only sha256 supported"));
    }
    let path = format!("{}/{}", data_dir(), name);
    let prio = req
        .query_params
        .get("prio")
        .map_or("normal", |s| s.as_str())
        .parse()
        .unwrap_or(WorkPriority::Normal);
    let timeout = worker_manager().io_timeout();
    let resp = worker_manager().submit_for_with_priority("/hashfile", timeout, prio, move || {
        let start = SystemTime::now();
        match std::fs::metadata(&path) {
            Ok(md) => {
                let size = md.len();
                match hashing::sha256_file_hex(&path) {
                    Ok(hex) => {
                        let elapsed = start.elapsed().unwrap_or_default().as_millis();
                        JsonResponseBuilder::new(200)
                            .field("filename", &path)
                            .field("algo", "sha256")
                            .field("hash", hex)
                            .field_num("size_bytes", size)
                            .field_num("elapsed_ms", elapsed)
                            .build()
                    }
                    Err(e) => JsonResponseBuilder::new(500)
                        .field("error", e.to_string())
                        .build(),
                }
            }
            Err(e) => {
                let code = if e.kind() == std::io::ErrorKind::NotFound { 404 } else { 500 };
                JsonResponseBuilder::new(code)
                    .field("error", e.to_string())
                    .build()
            }
        }
    })?;
    Ok(resp)
}

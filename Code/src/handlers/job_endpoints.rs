use std::collections::HashMap;

use crate::error::{ServerError, ServerResult};
use crate::jobs::job_manager::job_manager;
use crate::jobs::job_storage::CancelOutcome;
use crate::jobs::job_types::{JobStatus, now_ms};
use crate::server::requests::HttpRequest;
use crate::server::response::{HttpResponse, JsonResponseBuilder};

pub fn handle_job_submit(req: &HttpRequest) -> ServerResult<HttpResponse> {
    let route = req.require_query_param("route")?;
    // Build params by copying all query params except 'route'
    let mut params = HashMap::new();
    for (k, v) in req.query_params.iter() {
        if k != "route" { params.insert(k.clone(), v.clone()); }
    }
    let id = job_manager().submit(route, params)?;
    Ok(JsonResponseBuilder::new(202).field("job_id", id).build())
}

pub fn handle_job_status(req: &HttpRequest) -> ServerResult<HttpResponse> {
    let id = req.require_query_param("id")?;
    if let Some(j) = job_manager().status(id) {
        let (progress, eta_ms) = estimate_progress_eta(&j);
        let mut b = JsonResponseBuilder::new(200)
            .field("id", j.id)
            .field("route", j.route)
            .field("status", format_status(&j.status))
            .field_num("submitted_at", j.submitted_at)
            .field_num("progress", progress)
            .field_num("eta_ms", eta_ms);
        if let Some(s) = j.started_at { b = b.field_num("started_at", s); }
        if let Some(f) = j.finished_at { b = b.field_num("finished_at", f); }
        if let Some(rs) = j.result_status { b = b.field_num("result_status", rs as u64); }
        if let Some(err) = j.error { b = b.field("error", err); }
        Ok(b.build())
    } else {
        Err(ServerError::not_found(id))
    }
}

pub fn handle_job_result(req: &HttpRequest) -> ServerResult<HttpResponse> {
    let id = req.require_query_param("id")?;
    if let Some(j) = job_manager().status(id) {
        match j.result_raw {
            Some(bytes) => Ok(HttpResponse::ok().with_text(String::from_utf8_lossy(&bytes).to_string())),
            None => Err(ServerError::invalid_param("id", "result not ready")),
        }
    } else {
        Err(ServerError::not_found(id))
    }
}

pub fn handle_job_cancel(req: &HttpRequest) -> ServerResult<HttpResponse> {
    let id = req.require_query_param("id")?;
    match job_manager().cancel(id) {
        CancelOutcome::Canceled => Ok(JsonResponseBuilder::new(200)
            .field("id", id)
            .field_bool("canceled", true)
            .build()),
        CancelOutcome::NotFound => Ok(JsonResponseBuilder::new(404)
            .field("id", id)
            .field_bool("canceled", false)
            .field("error", "not_found")
            .build()),
        CancelOutcome::NotCancelable => Ok(JsonResponseBuilder::new(400)
            .field("id", id)
            .field_bool("canceled", false)
            .field("error", "not_cancelable")
            .build()),
    }
}

pub fn handle_job_list(_req: &HttpRequest) -> ServerResult<HttpResponse> {
    let jobs = job_manager().list();
    let mut arr = String::from("[");
    for (i, j) in jobs.iter().enumerate() {
        if i>0 { arr.push(','); }
        arr.push_str(&format!(
            r#"{{"id":"{}","route":"{}","status":"{}"}}"#,
            j.id, j.route, format_status(&j.status)
        ));
    }
    arr.push(']');
    Ok(JsonResponseBuilder::new(200).field_raw("jobs", arr).build())
}

fn format_status(s: &JobStatus) -> &'static str {
    match s {
        JobStatus::Queued => "queued",
        JobStatus::Running => "running",
        JobStatus::Done => "done",
        JobStatus::Failed => "failed",
        JobStatus::Canceled => "canceled",
    }
}

fn estimate_progress_eta(j: &crate::jobs::job_types::Job) -> (u64, u64) {
    match j.status {
        JobStatus::Done => return (100, 0),
        JobStatus::Failed | JobStatus::Canceled => return (100, 0),
        _ => {}
    }
    // Try to estimate from a seconds-like parameter
    let sec_value = j.params.get("seconds").or_else(|| j.params.get("sleep"));
    if let Some(sv) = sec_value {
        if let Ok(secs) = sv.parse::<u64>() {
            let total_ms = secs.saturating_mul(1000);
            if let Some(started) = j.started_at { // running
                let now = now_ms();
                let elapsed = now.saturating_sub(started) as u64;
                if total_ms == 0 { return (0, 0); }
                let pct = ((elapsed.saturating_mul(100)) / total_ms).min(99);
                let eta = total_ms.saturating_sub(elapsed);
                return (pct, eta);
            } else {
                // queued, unknown ETA
                return (0, 0);
            }
        }
    }
    // Fallback heuristics
    match j.status {
        JobStatus::Queued => (0, 0),
        JobStatus::Running => (50, 0),
        _ => (100, 0),
    }
}

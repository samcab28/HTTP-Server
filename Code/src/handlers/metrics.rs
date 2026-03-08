use crate::server::requests::HttpRequest;
use crate::server::response::{HttpResponse, JsonResponseBuilder};
use crate::error::ServerResult;
use crate::utils::metrics::metrics;
use crate::workers::worker_manager::worker_manager;

pub fn handle_metrics(_req: &HttpRequest) -> ServerResult<HttpResponse> {
    let snap = metrics().lock().unwrap().snapshot();
    let pool_stats = worker_manager().pool_stats();
    let mut items: Vec<String> = Vec::new();
    for (route, s) in snap.iter() {
        // find pool info if exists
        let mut qlen = 0usize; let mut qcap = 0usize; let mut workers = 0usize;
        let mut qh = 0usize; let mut qn = 0usize; let mut ql = 0usize;
        if let Some(ps) = pool_stats.iter().find(|p| p.route == *route) {
            qlen = ps.queue_len; qcap = ps.queue_capacity; workers = ps.workers;
            qh = ps.q_high; qn = ps.q_normal; ql = ps.q_low;
        }
        items.push(format!(
            r#"{{"route":"{}","requests":{},"successes":{},"failures":{},"s2xx":{},"s4xx":{},"s5xx":{},"avg_us":{},"min_us":{},"max_us":{},"queue_len":{},"queue_cap":{},"q_high":{},"q_normal":{},"q_low":{},"workers":{}}}"#,
            route,
            s.requests,
            s.successes,
            s.failures,
            s.s2xx,
            s.s4xx,
            s.s5xx,
            if s.requests>0 { s.total_us / s.requests as u128 } else { 0 },
            s.min_us,
            s.max_us,
            qlen,
            qcap,
            qh,
            qn,
            ql,
            workers
        ));
    }
    let json_routes = format!("[{}]", items.join(","));

    // Jobs queue snapshot (if present)
    let (jobs_size, jobs_high, jobs_normal, jobs_low) = crate::jobs::job_manager::job_manager().queue_counts();
    Ok(JsonResponseBuilder::new(200)
        .field_raw("routes", json_routes)
        .field_num("jobs_size", jobs_size as u64)
        .field_num("jobs_high", jobs_high as u64)
        .field_num("jobs_normal", jobs_normal as u64)
        .field_num("jobs_low", jobs_low as u64)
        .build())
}

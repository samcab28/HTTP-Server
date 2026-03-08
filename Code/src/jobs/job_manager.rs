use std::collections::HashMap;
use std::sync::{OnceLock};
use std::thread;

use crate::error::ServerResult;
use crate::handlers;
use crate::jobs::job_storage::{storage, CancelOutcome};
use crate::jobs::job_types::{Job, JobStatus, JobPriority, now_ms};
use crate::jobs::job_queue::JobQueue;
use crate::server::requests::{HttpMethod, HttpRequest};

pub struct JobManager {
    pub(crate) queue: JobQueue,
}

static MANAGER: OnceLock<JobManager> = OnceLock::new();

pub fn job_manager() -> &'static JobManager { MANAGER.get_or_init(|| JobManager { queue: JobQueue::with_capacity(1000) }) }

impl JobManager {
    /// Re-enqueue persisted jobs that were queued or running before shutdown.
    /// Running jobs are reset back to Queued and cleared of transient fields.
    pub fn recover_on_start(&self) {
        // Take a snapshot first to avoid holding the lock while pushing to queue
        let jobs_snapshot: Vec<Job> = { storage().lock().unwrap().list() };
        for mut j in jobs_snapshot.into_iter() {
            match j.status {
                JobStatus::Queued | JobStatus::Running => {
                    // Reset transient execution fields if previously running
                    if j.status == JobStatus::Running {
                        j.status = JobStatus::Queued;
                        j.started_at = None;
                        j.finished_at = None;
                        j.result_status = None;
                        j.result_raw = None;
                        j.error = None;
                    }
                    // Persist the reset (or keep queued) state
                    storage().lock().unwrap().update(j.clone());
                    // Enqueue back for execution with original priority
                    let _ = self.queue.push(j.route.clone(), j.id.clone(), j.priority);
                }
                _ => { /* Completed/Failed/Canceled: do not re-enqueue */ }
            }
        }
        // Ensure dispatcher is running
        Self::ensure_dispatcher();
    }
    pub fn submit(&self, route: &str, mut params: HashMap<String, String>) -> ServerResult<String> {
        let id = Self::new_id();
        let priority_param = params.get("priority").cloned().or_else(|| params.get("prio").cloned());
        let mut job = Job::new(id.clone(), route.to_string(), params.clone());
        // Optional scheduling params
        if let Some(p) = priority_param {
            job.priority = match p.to_lowercase().as_str() { "high" => JobPriority::High, "low" => JobPriority::Low, "normal" => JobPriority::Normal, _ => JobPriority::Normal };
            job.params.remove("priority");
            job.params.remove("prio");
        }
        if let Some(d) = params.remove("deadline_ms") { if let Ok(ms) = d.parse::<u128>() { job.deadline_ms = Some(now_ms()+ms); } }
        let priority_for_queue = job.priority;
        storage().lock().unwrap().insert(job);

        // Enqueue into scheduler
        let route_for_queue = route.to_string();
        let _ = self.queue.push(route_for_queue, id.clone(), priority_for_queue);

        // Ensure a dispatcher thread is running (spawn-once behavior)
        Self::ensure_dispatcher();

        Ok(id)
    }

    pub fn status(&self, id: &str) -> Option<Job> { storage().lock().unwrap().get(id) }
    pub fn list(&self) -> Vec<Job> { storage().lock().unwrap().list() }
    pub fn cancel(&self, id: &str) -> CancelOutcome { storage().lock().unwrap().cancel(id) }

    /// Expose jobs queue counts for metrics
    pub fn queue_counts(&self) -> (usize, usize, usize, usize) { self.queue.snapshot_counts() }

    fn new_id() -> String {
        // timestamp-counter
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let ts = now_ms();
        let c = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("{:016x}-{:08x}", ts, c)
    }

    fn ensure_dispatcher() {
        static STARTED: std::sync::Once = std::sync::Once::new();
        STARTED.call_once(|| {
            thread::spawn(move || loop {
                let jm = job_manager();
                if let Some((route, id)) = jm.queue.pop() {
                    // Pull a snapshot of the job without holding the mutex while processing handlers.
                    let job_snapshot = {
                        let st = storage().lock().unwrap();
                        st.get(&id)
                    };

                    if let Some(j) = job_snapshot {
                        // mark running
                        {
                            let mut st = storage().lock().unwrap();
                            if let Some(mut jj) = st.get(&id) { jj.status = JobStatus::Running; jj.started_at = Some(now_ms()); st.update(jj); }
                        }
                        // Build request and route
                        let req = HttpRequest { method: HttpMethod::Get, path: route.clone(), query_params: j.params.clone(), version: "HTTP/1.0".to_string(), headers: HashMap::new() };
                        let resp_result = match route.as_str() {
                            "/isprime" => handlers::handle_isprime(&req),
                            "/factor" => handlers::handle_factor(&req),
                            "/pi" => handlers::handle_pi(&req),
                            "/mandelbrot" => handlers::handle_mandelbrot(&req),
                            "/matrixmul" => handlers::handle_matrixmul(&req),
                            "/wordcount" => handlers::handle_wordcount(&req),
                            "/grep" => handlers::handle_grep(&req),
                            "/hashfile" => handlers::handle_hashfile(&req),
                            "/sortfile" => handlers::handle_sortfile(&req),
                            "/compress" => handlers::handle_compress(&req),
                            "/timestamp" => handlers::handle_timestamp(&req),
                            _ => Err(crate::error::ServerError::not_found(route.clone())),
                        };
                        let mut st = storage().lock().unwrap();
                        if let Some(mut jj) = st.get(&id) {
                            match resp_result {
                                Ok(resp) => { jj.result_status = Some(resp.status_code()); jj.result_raw = Some(resp.build()); jj.status = JobStatus::Done; }
                                Err(e) => { jj.error = Some(e.to_string()); jj.status = JobStatus::Failed; }
                            }
                            jj.finished_at = Some(now_ms());
                            st.update(jj);
                        }
                    }
                } else { break; }
            });
        });
    }
}

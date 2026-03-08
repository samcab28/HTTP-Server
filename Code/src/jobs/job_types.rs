use std::collections::HashMap;
use std::time::SystemTime;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Running,
    Done,
    Failed,
    Canceled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobPriority { Low, Normal, High }

#[derive(Clone, Debug)]
pub struct Job {
    pub id: String,
    pub route: String,
    pub params: HashMap<String, String>,
    pub status: JobStatus,
    pub submitted_at: u128,
    pub started_at: Option<u128>,
    pub finished_at: Option<u128>,
    pub result_status: Option<u16>,
    pub result_raw: Option<Vec<u8>>, // built HTTP bytes
    pub error: Option<String>,
    pub priority: JobPriority,
    pub deadline_ms: Option<u128>,
}

impl Job {
    pub fn new(id: String, route: String, params: HashMap<String,String>) -> Self {
        let now = now_ms();
        Self { id, route, params, status: JobStatus::Queued, submitted_at: now, started_at: None, finished_at: None, result_status: None, result_raw: None, error: None, priority: JobPriority::Normal, deadline_ms: None }
    }
}

pub fn now_ms() -> u128 {
    SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()
}

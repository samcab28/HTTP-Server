use std::time::Duration;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct CpuWorkerConfig {
    pub workers: usize,
    pub queue_depth: usize,
    pub timeout: Duration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkPriority {
    Low,
    Normal,
    High,
}

impl FromStr for WorkPriority {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "high" => WorkPriority::High,
            "low" => WorkPriority::Low,
            _ => WorkPriority::Normal,
        })
    }
}

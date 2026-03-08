use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

#[derive(Default, Clone, Copy)]
pub struct RouteStats {
    pub requests: u64,
    pub successes: u64,
    pub failures: u64,
    pub total_us: u128,
    pub min_us: u128,
    pub max_us: u128,
    pub s2xx: u64,
    pub s4xx: u64,
    pub s5xx: u64,
}

impl RouteStats {
    fn record(&mut self, ok: bool, us: u128) {
        self.requests += 1;
        if ok { self.successes += 1; } else { self.failures += 1; }
        self.total_us += us;
        if self.min_us == 0 || us < self.min_us { self.min_us = us; }
        if us > self.max_us { self.max_us = us; }
    }
}

#[derive(Default)]
pub struct Metrics {
    routes: HashMap<String, RouteStats>,
}

impl Metrics {
    pub fn record(&mut self, route: &str, ok: bool, dur: Duration) {
        let entry = self.routes.entry(route.to_string()).or_default();
        entry.record(ok, dur.as_micros());
    }

    pub fn record_status(&mut self, route: &str, status: u16, dur: Duration) {
        let entry = self.routes.entry(route.to_string()).or_default();
        let ok = status < 400;
        entry.record(ok, dur.as_micros());
        match status {
            200..=299 => entry.s2xx += 1,
            400..=499 => entry.s4xx += 1,
            500..=599 => entry.s5xx += 1,
            _ => {}
        }
    }

    pub fn snapshot(&self) -> HashMap<String, RouteStats> {
        self.routes.clone()
    }
}

static METRICS: OnceLock<Mutex<Metrics>> = OnceLock::new();

pub fn metrics() -> &'static Mutex<Metrics> {
    METRICS.get_or_init(|| Mutex::new(Metrics::default()))
}

use std::collections::HashMap;
use std::sync::{OnceLock, Mutex};
use std::time::Duration;
use std::sync::mpsc::{self, Receiver, Sender};

use crate::config::ServerConfig;
use crate::error::{ServerError, ServerResult};

use super::worker_pool::{Job, WorkerPool};
use super::worker_types::{CpuWorkerConfig, WorkPriority};

pub struct WorkerManager {
    cpu_pool: WorkerPool,
    cpu_timeout: Duration,
    io_pool: WorkerPool,
    io_timeout: Duration,
    endpoint_pools: Mutex<HashMap<String, WorkerPool>>,
    config: ServerConfig,
}

impl WorkerManager {
    pub fn new(config: &ServerConfig) -> Self {
        // Configure CPU pool parameters from ServerConfig
        let workers = config.workers.get_workers("cpu");
        let depth = config.queues.get_depth("cpu");
        let timeout = config.timeouts.cpu_timeout;

        let cpu_cfg = CpuWorkerConfig { workers, queue_depth: depth, timeout };

        let cpu_pool = WorkerPool::new("cpu", cpu_cfg.workers, cpu_cfg.queue_depth);

        // IO pool mirrors CPU settings by default but uses IO timeout and separate sizing
        let io_workers = config.workers.get_workers("io");
        let io_depth = config.queues.get_depth("io");
        let io_timeout = config.timeouts.io_timeout;
        let io_pool = WorkerPool::new("io", io_workers, io_depth);

        Self {
            cpu_pool,
            cpu_timeout: cpu_cfg.timeout,
            io_pool,
            io_timeout,
            endpoint_pools: Mutex::new(HashMap::new()),
            config: config.clone(),
        }
    }

    pub fn cpu_timeout(&self) -> Duration { self.cpu_timeout }
    pub fn io_timeout(&self) -> Duration { self.io_timeout }

    /// Submit a CPU task that returns a value via oneshot channel.
    pub fn submit_cpu<F, R>(&self, f: F) -> ServerResult<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (tx, rx): (Sender<R>, Receiver<R>) = mpsc::channel();

        let job: Job = Box::new(move || {
            let result = f();
            // Ignore send errors if receiver dropped
            let _ = tx.send(result);
        });

        self.cpu_pool.submit(job)?;

        match rx.recv_timeout(self.cpu_timeout) {
            Ok(value) => Ok(value),
            Err(mpsc::RecvTimeoutError::Timeout) => Err(ServerError::Timeout),
            Err(_) => Err(ServerError::internal("worker channel closed")),
        }
    }

    /// Submit an IO task that returns a value via oneshot channel.
    pub fn submit_io<F, R>(&self, f: F) -> ServerResult<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (tx, rx): (Sender<R>, Receiver<R>) = mpsc::channel();

        let job: Job = Box::new(move || {
            let result = f();
            let _ = tx.send(result);
        });

        self.io_pool.submit(job)?;

        match rx.recv_timeout(self.io_timeout) {
            Ok(value) => Ok(value),
            Err(mpsc::RecvTimeoutError::Timeout) => Err(ServerError::Timeout),
            Err(_) => Err(ServerError::internal("worker channel closed")),
        }
    }

    /// Submit to a route-specific pool with the provided timeout.
    pub fn submit_for<F, R>(&self, route: &str, timeout: Duration, f: F) -> ServerResult<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        // Ensure pool exists
        {
            let mut map = self.endpoint_pools.lock().unwrap();
            if !map.contains_key(route) {
                let workers = self.config.workers.get_workers(route);
                let depth = self.config.queues.get_depth(route);
                let pool = WorkerPool::new(route.to_string(), workers, depth);
                map.insert(route.to_string(), pool);
            }
        }

        let (tx, rx): (Sender<R>, Receiver<R>) = mpsc::channel();
        let job: Job = Box::new(move || {
            let result = f();
            let _ = tx.send(result);
        });

        // Submit without holding lock during execution
        {
            let map = self.endpoint_pools.lock().unwrap();
            let pool = map.get(route).expect("pool must exist");
            pool.submit(job)?;
        }

        let effective_timeout = self.config.timeouts.get_for_route(route).unwrap_or(timeout);
        match rx.recv_timeout(effective_timeout) {
            Ok(value) => Ok(value),
            Err(mpsc::RecvTimeoutError::Timeout) => Err(ServerError::Timeout),
            Err(_) => Err(ServerError::internal("worker channel closed")),
        }
    }

    /// Submit to a route-specific pool with explicit priority.
    pub fn submit_for_with_priority<F, R>(&self, route: &str, timeout: Duration, prio: WorkPriority, f: F) -> ServerResult<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        // Ensure pool exists
        {
            let mut map = self.endpoint_pools.lock().unwrap();
            if !map.contains_key(route) {
                let workers = self.config.workers.get_workers(route);
                let depth = self.config.queues.get_depth(route);
                let pool = WorkerPool::new(route.to_string(), workers, depth);
                map.insert(route.to_string(), pool);
            }
        }

        let (tx, rx): (Sender<R>, Receiver<R>) = mpsc::channel();
        let job: Job = Box::new(move || {
            let result = f();
            let _ = tx.send(result);
        });

        {
            let map = self.endpoint_pools.lock().unwrap();
            let pool = map.get(route).expect("pool must exist");
            pool.submit_with_priority(job, prio)?;
        }

        let effective_timeout = self.config.timeouts.get_for_route(route).unwrap_or(timeout);
        match rx.recv_timeout(effective_timeout) {
            Ok(value) => Ok(value),
            Err(mpsc::RecvTimeoutError::Timeout) => Err(ServerError::Timeout),
            Err(_) => Err(ServerError::internal("worker channel closed")),
        }
    }

    /// Snapshot of current endpoint pools with queue/worker stats
    pub fn pool_stats(&self) -> Vec<RoutePoolStats> {
        let mut out = Vec::new();
        let map = self.endpoint_pools.lock().unwrap();
        for (route, pool) in map.iter() {
            let (qh, qn, ql) = pool.queue_len_per_prio();
            out.push(RoutePoolStats {
                route: route.clone(),
                workers: pool.workers_count(),
                queue_len: pool.queue_len(),
                queue_capacity: pool.queue_capacity(),
                q_high: qh,
                q_normal: qn,
                q_low: ql,
            });
        }
        out
    }
}

pub struct RoutePoolStats {
    pub route: String,
    pub workers: usize,
    pub queue_len: usize,
    pub queue_capacity: usize,
    pub q_high: usize,
    pub q_normal: usize,
    pub q_low: usize,
}

static GLOBAL_MANAGER: OnceLock<WorkerManager> = OnceLock::new();

pub fn init_global_worker_manager(config: &ServerConfig) {
    let _ = GLOBAL_MANAGER.set(WorkerManager::new(config));
}

pub fn worker_manager() -> &'static WorkerManager {
    GLOBAL_MANAGER.get_or_init(|| WorkerManager::new(&ServerConfig::default()))
}

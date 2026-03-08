use crate::error::{ServerError, ServerResult};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc};
use std::thread::{JoinHandle, Builder};

use super::task_queue::TaskQueue;
use super::worker_types::WorkPriority;

pub type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct WorkerPool {
    name: String,
    queue: Arc<TaskQueue<Job>>,
    workers: Vec<JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
}

impl WorkerPool {
    pub fn new(name: impl Into<String>, size: usize, capacity: usize) -> Self {
        let name = name.into();
        let queue: Arc<TaskQueue<Job>> = Arc::new(TaskQueue::<Job>::with_capacity(capacity));
        let shutdown = Arc::new(AtomicBool::new(false));

        let mut workers = Vec::with_capacity(size);
        for i in 0..size {
            let queue = Arc::clone(&queue);
            let shutdown = Arc::clone(&shutdown);
            let worker_name = format!("{}-{}", &name, i);
            let handle = Builder::new().name(worker_name).spawn(move || {
                while !shutdown.load(Ordering::SeqCst) {
                    match queue.pop() {
                        Some(job) => {
                            // Execute the job; any panic only kills this thread
                            job();
                        }
                        None => {
                            // Queue closed; exit
                            break;
                        }
                    }
                }
            }).expect("failed to spawn worker thread");
            workers.push(handle);
        }

        Self {
            name,
            queue,
            workers,
            shutdown,
        }
    }

    pub fn submit(&self, job: Job) -> ServerResult<()> {
        if self.shutdown.load(Ordering::SeqCst) {
            return Err(ServerError::ResourceExhausted(format!(
                "pool {} shutting down",
                self.name
            )));
        }
        if !self.queue.try_push(job) {
            return Err(ServerError::ResourceExhausted(format!(
                "pool {} queue full",
                self.name
            )));
        }
        Ok(())
    }

    pub fn submit_with_priority(&self, job: Job, prio: WorkPriority) -> ServerResult<()> {
        if self.shutdown.load(Ordering::SeqCst) {
            return Err(ServerError::ResourceExhausted(format!("pool {} shutting down", self.name)));
        }
        if !self.queue.try_push_with_priority(job, prio) {
            return Err(ServerError::ResourceExhausted(format!("pool {} queue full", self.name)));
        }
        Ok(())
    }

    pub fn shutdown(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        self.queue.close();
        // Join all workers
        for handle in self.workers.drain(..) {
            let _ = handle.join();
        }
    }

    pub fn queue_len(&self) -> usize { self.queue.len() }
    pub fn queue_capacity(&self) -> usize { self.queue.capacity() }
    pub fn workers_count(&self) -> usize { self.workers.len() }

    pub fn queue_len_per_prio(&self) -> (usize, usize, usize) { self.queue.len_per_priority() }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        self.shutdown();
    }
}

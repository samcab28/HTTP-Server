use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, Condvar};

use super::job_types::JobPriority;

/// Round-robin job queue across routes. Each route has a FIFO queue of job IDs.
pub struct JobQueue {
    inner: Mutex<Inner>,
    cv: Condvar,
}

struct Inner {
    per_route: HashMap<String, RouteQueues>,
    active_routes: PriorityTracker,
    capacity: usize,
    size: usize,
    closed: bool,
}

#[derive(Default)]
struct RouteQueues {
    high: VecDeque<String>,
    normal: VecDeque<String>,
    low: VecDeque<String>,
}

impl RouteQueues {
    fn push(&mut self, priority: JobPriority, id: String) {
        match priority {
            JobPriority::High => self.high.push_back(id),
            JobPriority::Normal => self.normal.push_back(id),
            JobPriority::Low => self.low.push_back(id),
        }
    }

    fn pop(&mut self, priority: JobPriority) -> Option<String> {
        match priority {
            JobPriority::High => self.high.pop_front(),
            JobPriority::Normal => self.normal.pop_front(),
            JobPriority::Low => self.low.pop_front(),
        }
    }

    fn has_priority(&self, priority: JobPriority) -> bool {
        !self.is_empty_priority(priority)
    }

    fn is_empty_priority(&self, priority: JobPriority) -> bool {
        match priority {
            JobPriority::High => self.high.is_empty(),
            JobPriority::Normal => self.normal.is_empty(),
            JobPriority::Low => self.low.is_empty(),
        }
    }

    fn is_empty(&self) -> bool {
        self.high.is_empty() && self.normal.is_empty() && self.low.is_empty()
    }
}

#[derive(Default)]
struct PriorityTracker {
    lists: [Vec<String>; 3],
    indices: [usize; 3],
}

impl PriorityTracker {
    fn add_route(&mut self, priority: JobPriority, route: &str) {
        let idx = priority_index(priority);
        let list = &mut self.lists[idx];
        if !list.iter().any(|r| r == route) {
            list.push(route.to_string());
        }
    }

    fn remove_route(&mut self, priority: JobPriority, route: &str) {
        let idx = priority_index(priority);
        let list = &mut self.lists[idx];
        if let Some(pos) = list.iter().position(|r| r == route) {
            list.remove(pos);
            let rr = &mut self.indices[idx];
            if *rr > pos {
                *rr -= 1;
            }
            if list.is_empty() || *rr >= list.len() {
                *rr = 0;
            }
        }
    }

    fn remove_all(&mut self, route: &str) {
        for priority in [JobPriority::High, JobPriority::Normal, JobPriority::Low] {
            self.remove_route(priority, route);
        }
    }

    fn next_route(&mut self) -> Option<(JobPriority, String)> {
        for priority in [JobPriority::High, JobPriority::Normal, JobPriority::Low] {
            let idx = priority_index(priority);
            let list = &mut self.lists[idx];
            if list.is_empty() { continue; }
            let rr = &mut self.indices[idx];
            if *rr >= list.len() {
                *rr = 0;
            }
            let route = list[*rr].clone();
            *rr = (*rr + 1) % list.len();
            return Some((priority, route));
        }
        None
    }

    fn rebuild_from_entries(&mut self, entries: &[(JobPriority, String)]) {
        for list in &mut self.lists { list.clear(); }
        for idx in &mut self.indices { *idx = 0; }
        for (priority, route) in entries {
            self.add_route(*priority, route);
        }
    }

    fn has_any(&self) -> bool {
        self.lists.iter().any(|list| !list.is_empty())
    }
}

fn priority_index(priority: JobPriority) -> usize {
    match priority {
        JobPriority::High => 0,
        JobPriority::Normal => 1,
        JobPriority::Low => 2,
    }
}

impl JobQueue {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(Inner {
                per_route: HashMap::new(),
                active_routes: PriorityTracker::default(),
                capacity,
                size: 0,
                closed: false,
            }),
            cv: Condvar::new(),
        }
    }

    /// Enqueue a job id under a route. Returns Err if global capacity is full.
    pub fn push(&self, route: String, id: String, priority: JobPriority) -> Result<(), ()> {
        let mut inner = self.inner.lock().unwrap();
        if inner.size >= inner.capacity { return Err(()); }
        let queues = inner.per_route.entry(route.clone()).or_default();
        let priority_was_empty = queues.is_empty_priority(priority);
        queues.push(priority, id);
        inner.size += 1;
        if priority_was_empty {
            inner.active_routes.add_route(priority, &route);
        }
        self.cv.notify_one();
        Ok(())
    }

    /// Pop next (route, id) in round-robin order.
    pub fn pop(&self) -> Option<(String, String)> {
        let mut inner = self.inner.lock().unwrap();
        loop {
            if inner.closed { return None; }
            if inner.size == 0 {
                inner = self.cv.wait(inner).unwrap();
                continue;
            }
            if let Some((priority, route)) = inner.active_routes.next_route() {
                let (id_opt, priority_empty, route_empty) = {
                    let queues = inner.per_route.get_mut(&route);
                    if let Some(q) = queues {
                        let id = q.pop(priority);
                        let priority_empty = q.is_empty_priority(priority);
                        let route_empty = q.is_empty();
                        (id, priority_empty, route_empty)
                    } else {
                        (None, true, true)
                    }
                };

                match id_opt {
                    Some(id) => {
                        inner.size -= 1;
                        if route_empty {
                            inner.active_routes.remove_all(&route);
                            inner.per_route.remove(&route);
                        } else if priority_empty {
                            inner.active_routes.remove_route(priority, &route);
                        }
                        return Some((route, id));
                    }
                    None => {
                        if route_empty {
                            inner.active_routes.remove_all(&route);
                            inner.per_route.remove(&route);
                        } else if priority_empty {
                            inner.active_routes.remove_route(priority, &route);
                        }
                        // loop again to find another job
                        continue;
                    }
                }
            } else {
                let mut snapshot = Vec::new();
                for (route, queues) in inner.per_route.iter() {
                    if queues.has_priority(JobPriority::High) {
                        snapshot.push((JobPriority::High, route.clone()));
                    }
                    if queues.has_priority(JobPriority::Normal) {
                        snapshot.push((JobPriority::Normal, route.clone()));
                    }
                    if queues.has_priority(JobPriority::Low) {
                        snapshot.push((JobPriority::Low, route.clone()));
                    }
                }
                inner.active_routes.rebuild_from_entries(&snapshot);
                if inner.active_routes.has_any() {
                    continue;
                } else {
                    inner = self.cv.wait(inner).unwrap();
                }
            }
        }
    }

    pub fn close(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.closed = true;
        self.cv.notify_all();
    }

    /// Snapshot counts across all routes by priority and total size
    pub fn snapshot_counts(&self) -> (usize, usize, usize, usize) {
        let inner = self.inner.lock().unwrap();
        let mut high = 0usize; let mut normal = 0usize; let mut low = 0usize;
        for (_route, queues) in inner.per_route.iter() {
            high += queues.high.len();
            normal += queues.normal.len();
            low += queues.low.len();
        }
        (inner.size, high, normal, low)
    }
}

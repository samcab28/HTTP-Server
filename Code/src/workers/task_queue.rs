use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};

pub struct TaskQueue<T> {
    inner: Mutex<Inner<T>>,
    cv: Condvar,
}

struct Inner<T> {
    high: VecDeque<T>,
    normal: VecDeque<T>,
    low: VecDeque<T>,
    capacity: usize,
    size: usize,
    closed: bool,
}

impl<T> TaskQueue<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(Inner {
                high: VecDeque::new(),
                normal: VecDeque::new(),
                low: VecDeque::new(),
                capacity,
                size: 0,
                closed: false,
            }),
            cv: Condvar::new(),
        }
    }

    /// Try to push without blocking. Returns false if full or closed.
    pub fn try_push(&self, item: T) -> bool {
        self.try_push_with_priority(item, crate::workers::worker_types::WorkPriority::Normal)
    }

    /// Push with explicit priority
    pub fn try_push_with_priority(&self, item: T, prio: crate::workers::worker_types::WorkPriority) -> bool {
        let mut inner = self.inner.lock().unwrap();
        if inner.closed || inner.size >= inner.capacity { return false; }
        match prio {
            crate::workers::worker_types::WorkPriority::High => inner.high.push_back(item),
            crate::workers::worker_types::WorkPriority::Normal => inner.normal.push_back(item),
            crate::workers::worker_types::WorkPriority::Low => inner.low.push_back(item),
        }
        inner.size += 1;
        self.cv.notify_one();
        true
    }

    /// Pop blocking; returns None when closed and empty.
    pub fn pop(&self) -> Option<T> {
        let mut inner = self.inner.lock().unwrap();
        loop {
            if let Some(item) = inner.high.pop_front() { inner.size -= 1; return Some(item); }
            if let Some(item) = inner.normal.pop_front() { inner.size -= 1; return Some(item); }
            if let Some(item) = inner.low.pop_front() { inner.size -= 1; return Some(item); }
            if inner.closed {
                return None;
            }
            inner = self.cv.wait(inner).unwrap();
        }
    }

    pub fn close(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.closed = true;
        self.cv.notify_all();
    }

    /// Current queue length
    pub fn len(&self) -> usize {
        let inner = self.inner.lock().unwrap();
        inner.size
    }

    /// Check whether the queue is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Queue capacity
    pub fn capacity(&self) -> usize {
        let inner = self.inner.lock().unwrap();
        inner.capacity
    }

    /// Per-priority queue lengths (high, normal, low)
    pub fn len_per_priority(&self) -> (usize, usize, usize) {
        let inner = self.inner.lock().unwrap();
        (inner.high.len(), inner.normal.len(), inner.low.len())
    }
}

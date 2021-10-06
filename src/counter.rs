use std::sync::atomic::{AtomicI32, Ordering};

pub struct Counter {
    count: AtomicI32
}

impl Counter {
    pub const fn new() -> Self {
        Self {
            count: AtomicI32::new(0)
        }
    }

    pub fn get(&self) -> i32 {
        self.count.load(Ordering::SeqCst)
    }

    pub fn inc(&self) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }

    /// boost the counter
    /// (increment the counter, but return a guard that will decrement it when dropped)
    pub fn boost(&self) -> DecrementGuard<'_> {
        let guard = DecrementGuard { count: &self.count };
        self.count.fetch_add(1, Ordering::SeqCst);
        guard
    }
}

pub struct DecrementGuard<'a> {
    count: &'a AtomicI32,
}

impl Drop for DecrementGuard<'_> {
    fn drop(&mut self) {
        self.count.fetch_sub(1, Ordering::SeqCst);
    }
}

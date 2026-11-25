//! Shared execution slots for the native and LLVM workers of one compiler child.

use std::num::NonZeroUsize;
use std::sync::{Arc, Condvar, Mutex};

#[derive(Clone, Debug)]
pub struct WorkerSlots(Arc<Slots>);

#[derive(Debug)]
struct Slots {
    limit: NonZeroUsize,
    available: Mutex<usize>,
    released: Condvar,
}

#[derive(Debug)]
pub struct WorkerSlot(WorkerSlots);

impl WorkerSlots {
    pub fn new(limit: NonZeroUsize) -> Self {
        Self(Arc::new(Slots {
            limit,
            available: Mutex::new(limit.get()),
            released: Condvar::new(),
        }))
    }

    pub fn limit(&self) -> NonZeroUsize {
        self.0.limit
    }

    pub fn acquire(&self) -> WorkerSlot {
        let mut available = self
            .0
            .available
            .lock()
            .expect("worker slot lock is not poisoned");
        while *available == 0 {
            available = self
                .0
                .released
                .wait(available)
                .expect("worker slot lock is not poisoned");
        }
        *available -= 1;
        WorkerSlot(self.clone())
    }
}

impl From<NonZeroUsize> for WorkerSlots {
    fn from(limit: NonZeroUsize) -> Self {
        Self::new(limit)
    }
}

impl From<&WorkerSlots> for WorkerSlots {
    fn from(slots: &WorkerSlots) -> Self {
        slots.clone()
    }
}

impl Drop for WorkerSlot {
    fn drop(&mut self) {
        let mut available = self
            .0
            .0
            .available
            .lock()
            .expect("worker slot lock is not poisoned");
        *available += 1;
        self.0.0.released.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn independent_worker_groups_share_one_concurrency_limit() {
        for limit in [1, 2, 4] {
            let slots = WorkerSlots::new(NonZeroUsize::new(limit).unwrap());
            let start = Barrier::new(8);
            let running = AtomicUsize::new(0);
            let maximum = AtomicUsize::new(0);
            std::thread::scope(|scope| {
                for _ in 0..8 {
                    let slots = slots.clone();
                    let (start, running, maximum) = (&start, &running, &maximum);
                    scope.spawn(move || {
                        start.wait();
                        for _ in 0..100 {
                            let _slot = slots.acquire();
                            let active = running.fetch_add(1, Ordering::SeqCst) + 1;
                            maximum.fetch_max(active, Ordering::SeqCst);
                            std::thread::yield_now();
                            running.fetch_sub(1, Ordering::SeqCst);
                        }
                    });
                }
            });
            assert_eq!(running.load(Ordering::SeqCst), 0);
            assert!(maximum.load(Ordering::SeqCst) <= limit);
        }
    }
}

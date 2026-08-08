use crate::hint::spin_loop;
use crate::sync::atomic::{AtomicUsize, Ordering, fence};

use super::api::BoundedRawLock;

pub struct FilterLock {
    level: Vec<AtomicUsize>,
    victim: Vec<AtomicUsize>,
    n: usize,
}

impl BoundedRawLock for FilterLock {
    fn new(n: usize) -> Self {
        Self {
            level: (0..n).map(|_| AtomicUsize::new(0)).collect(),
            victim: (0..n).map(|_| AtomicUsize::new(0)).collect(),
            n,
        }
    }
    fn lock(&self, id: usize) {
        assert!(id < self.n);

        for i in 1..self.n {
            self.level[id].store(i, Ordering::Relaxed);
            fence(Ordering::SeqCst);
            self.victim[i].store(id, Ordering::Relaxed);
            fence(Ordering::SeqCst);
            for k in 0..self.n {
                if id == k {
                    continue;
                }
                while self.level[k].load(Ordering::Acquire) >= i
                    && self.victim[i].load(Ordering::Relaxed) == id
                {
                    spin_loop();
                }
            }
        }
    }
    unsafe fn unlock(&self, id: usize) {
        self.level[id].store(0, Ordering::Release);
    }
}

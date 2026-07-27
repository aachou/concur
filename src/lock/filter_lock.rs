use crate::hint::spin_loop;
use crate::sync::atomic::{AtomicUsize, Ordering, fence};

#[derive(Debug)]
pub struct FilterLock {
    n: usize,
    level: Vec<AtomicUsize>,
    victim: Vec<AtomicUsize>,
}

impl FilterLock {
    pub fn new(n: usize) -> Self {
        let mut level = Vec::with_capacity(n);
        let mut victim = Vec::with_capacity(n);
        for _ in 0..n {
            level.push(AtomicUsize::new(0));
            victim.push(AtomicUsize::new(0));
        }
        Self { n, level, victim }
    }

    pub fn lock(&self, id: usize) {
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

    pub fn unlock(&self, id: usize) {
        self.level[id].store(0, Ordering::Release);
    }
}

use crate::hint::spin_loop;
use crate::sync::atomic::{AtomicBool, AtomicUsize, Ordering, fence};

use super::api::BoundedRawLock;

pub struct PetersonLock {
    flag: [AtomicBool; 2],
    victim: AtomicUsize,
    n: usize,
}

impl BoundedRawLock for PetersonLock {
    fn new(n: usize) -> Self {
        assert!(n <= 2);

        Self {
            flag: [AtomicBool::new(false), AtomicBool::new(false)],
            victim: AtomicUsize::new(0),
            n,
        }
    }
    unsafe fn lock(&self, id: usize) {
        assert!(id < self.n);

        self.flag[id].store(true, Ordering::Relaxed);
        fence(Ordering::SeqCst);
        self.victim.store(id, Ordering::Relaxed);
        fence(Ordering::SeqCst);
        while self.flag[1 - id].load(Ordering::Acquire) && self.victim.load(Ordering::Relaxed) == id
        {
            spin_loop();
        }
    }
    unsafe fn unlock(&self, id: usize) {
        self.flag[id].store(false, Ordering::Release);
    }
}

use crate::hint::spin_loop;
use crate::sync::atomic::{AtomicBool, AtomicUsize, Ordering, fence};

#[derive(Debug, Default)]
pub struct PetersonLock {
    flag: [AtomicBool; 2],
    victim: AtomicUsize,
}

impl PetersonLock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn lock(&self, id: usize) {
        self.flag[id].store(true, Ordering::Relaxed);
        fence(Ordering::SeqCst);
        self.victim.store(id, Ordering::Relaxed);
        fence(Ordering::SeqCst);
        while self.flag[1 - id].load(Ordering::Acquire) && self.victim.load(Ordering::Relaxed) == id
        {
            spin_loop();
        }
    }

    pub fn unlock(&self, id: usize) {
        self.flag[id].store(false, Ordering::Release);
    }
}

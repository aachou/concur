use crate::hint::spin_loop;
use crate::sync::atomic::{AtomicBool, AtomicUsize, Ordering, fence};

use super::api::BoundedRawLock;

pub struct BakeryLock {
    flag: Vec<AtomicBool>,
    label: Vec<AtomicUsize>,
    n: usize,
}

impl BoundedRawLock for BakeryLock {
    fn new(n: usize) -> Self {
        Self {
            flag: (0..n).map(|_| AtomicBool::new(false)).collect(),
            label: (0..n).map(|_| AtomicUsize::new(0)).collect(),
            n,
        }
    }

    unsafe fn lock(&self, id: usize) {
        assert!(id < self.n);

        self.flag[id].store(true, Ordering::Relaxed);
        fence(Ordering::SeqCst);
        self.label[id].store(
            self.label
                .iter()
                .map(|x| x.load(Ordering::Relaxed))
                .max()
                .unwrap()
                + 1,
            Ordering::Relaxed,
        );
        fence(Ordering::SeqCst);
        for k in 0..self.n {
            while self.flag[k].load(Ordering::Acquire)
                && (self.label[k].load(Ordering::Relaxed), k)
                    < (self.label[id].load(Ordering::Relaxed), id)
            {
                spin_loop();
            }
        }
    }

    unsafe fn unlock(&self, id: usize) {
        self.flag[id].store(false, Ordering::Release);
    }
}

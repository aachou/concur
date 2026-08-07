use crate::cell::UnsafeCell;
use crate::sync::atomic::{Ordering, fence};
use crate::sync::{Mutex, MutexGuard};

use super::api::{Consensus, ConsensusProtocol};

struct Assign {
    inner: Mutex<Vec<i32>>,
    n: usize,
}

const NULL: i32 = -1;

impl Assign {
    fn new(n: usize) -> Self {
        Self {
            inner: Mutex::new(vec![NULL; n * (n + 1) / 2]),
            n,
        }
    }

    fn index(&self, i: usize, j: usize) -> usize {
        self.n - 1 + j - i
            + if i > 0 {
                (2 * self.n - 1 - i) * i / 2
            } else {
                0
            }
    }

    fn assign(&self, id: usize, value: i32) {
        let mut guard = self.inner.lock().unwrap();
        guard[id] = value;
        for j in (id + 1)..self.n {
            guard[self.index(id, j)] = value;
        }
        for i in 0..id {
            guard[self.index(i, id)] = value;
        }
    }
    fn read(&self) -> MutexGuard<'_, Vec<i32>> {
        self.inner.lock().unwrap()
    }
}

pub struct MultiConsensus<T> {
    proposed: Vec<UnsafeCell<Option<T>>>,
    assign: Assign,
    n: usize,
}

impl<T: Clone> Consensus<T> for MultiConsensus<T> {
    fn new(n: usize) -> Self {
        let mut proposed = Vec::with_capacity(n);
        for _ in 0..n {
            proposed.push(UnsafeCell::new(None));
        }
        Self {
            proposed,
            assign: Assign::new(n),
            n,
        }
    }
    fn decide(&self, value: T, id: usize) -> T {
        assert!(id < self.n);

        self.propose(value, id);

        fence(Ordering::SeqCst);

        self.assign.assign(id, id as i32);

        let guard = self.assign.read();

        let mut win = 0;
        for j in 1..self.n {
            if guard[j] == NULL {
                continue;
            }
            if guard[win] == NULL || guard[win] == guard[self.assign.index(win, j)] {
                win = j;
            }
        }

        #[cfg(not(feature = "check-loom"))]
        unsafe {
            (*self.proposed[win].get()).clone().unwrap()
        }

        #[cfg(feature = "check-loom")]
        unsafe {
            (*self.proposed[win].get().deref()).clone().unwrap()
        }
    }
}

impl<T: Clone> ConsensusProtocol<T> for MultiConsensus<T> {
    fn propose(&self, value: T, id: usize) {
        #[cfg(not(feature = "check-loom"))]
        unsafe {
            *self.proposed[id].get() = Some(value);
        }

        #[cfg(feature = "check-loom")]
        unsafe {
            *self.proposed[id].get_mut().deref() = Some(value);
        }
    }
}

unsafe impl<T: Send> Send for MultiConsensus<T> {}
unsafe impl<T: Sync> Sync for MultiConsensus<T> {}

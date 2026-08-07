use crate::cell::UnsafeCell;
use crate::sync::Mutex;
use crate::sync::atomic::{Ordering, fence};

use super::api::{Consensus, ConsensusProtocol};

struct Queue<T> {
    inner: Mutex<Vec<T>>,
}

impl<T> Queue<T> {
    fn new() -> Self {
        Self {
            inner: Mutex::new(Vec::new()),
        }
    }
    fn enq(&self, item: T) {
        self.inner.lock().unwrap().push(item);
    }
    fn deq(&self) -> T {
        self.inner.lock().unwrap().remove(0)
    }
}

pub struct QueueConsensus<T> {
    proposed: [UnsafeCell<Option<T>>; 2],
    queue: Queue<usize>,
    n: usize,
}

const WIN: usize = 0;
const LOSE: usize = 1;

impl<T: Clone> Consensus<T> for QueueConsensus<T> {
    fn new(n: usize) -> Self {
        assert!(n <= 2);

        let queue = Queue::new();
        queue.enq(WIN);
        queue.enq(LOSE);

        Self {
            proposed: [UnsafeCell::new(None), UnsafeCell::new(None)],
            queue,
            n,
        }
    }

    fn decide(&self, value: T, id: usize) -> T {
        assert!(id < self.n);

        self.propose(value, id);

        fence(Ordering::SeqCst);

        #[cfg(not(feature = "check-loom"))]
        unsafe {
            if self.queue.deq() == WIN {
                (*self.proposed[id].get()).clone().unwrap()
            } else {
                (*self.proposed[1 - id].get()).clone().unwrap()
            }
        }

        #[cfg(feature = "check-loom")]
        unsafe {
            if self.queue.deq() == WIN {
                (*self.proposed[id].get().deref()).clone().unwrap()
            } else {
                (*self.proposed[1 - id].get().deref()).clone().unwrap()
            }
        }
    }
}

impl<T: Clone> ConsensusProtocol<T> for QueueConsensus<T> {
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

unsafe impl<T: Send> Send for QueueConsensus<T> {}
unsafe impl<T: Sync> Sync for QueueConsensus<T> {}

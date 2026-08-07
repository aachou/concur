use crate::cell::UnsafeCell;
use crate::sync::Mutex;

use super::api::{Consensus, ConsensusProtocol, cell_read, cell_write};

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

unsafe impl<T: Send> Send for Queue<T> {}
unsafe impl<T: Send> Sync for Queue<T> {}

pub struct QueueConsensus<T> {
    proposed: [UnsafeCell<Option<T>>; 2],
    queue: Queue<usize>,
}

const WIN: usize = 0;
const LOSE: usize = 1;

impl<T> QueueConsensus<T> {
    pub fn new(n: usize) -> Self {
        assert!(n <= 2);

        let queue = Queue::new();
        queue.enq(WIN);
        queue.enq(LOSE);

        Self {
            proposed: [UnsafeCell::new(None), UnsafeCell::new(None)],
            queue,
        }
    }
}

impl<T> Default for QueueConsensus<T> {
    fn default() -> Self {
        Self::new(2)
    }
}

impl<T: Clone> Consensus<T> for QueueConsensus<T> {
    fn decide(&self, value: T, id: usize) -> T {
        assert!(id < self.proposed.len());

        self.propose(value, id);

        if self.queue.deq() == WIN {
            cell_read(&self.proposed[id]).unwrap()
        } else {
            cell_read(&self.proposed[1 - id]).unwrap()
        }
    }
}

impl<T: Clone> ConsensusProtocol<T> for QueueConsensus<T> {
    fn propose(&self, value: T, id: usize) {
        cell_write(&self.proposed[id], Some(value));
    }
}

unsafe impl<T: Send> Send for QueueConsensus<T> {}
unsafe impl<T: Sync> Sync for QueueConsensus<T> {}

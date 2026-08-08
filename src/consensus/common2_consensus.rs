use crate::cell::UnsafeCell;
use crate::sync::atomic::{AtomicUsize, Ordering};

use super::api::{Consensus, ConsensusProtocol, cell_read, cell_write};

pub struct Common2Consensus<T> {
    proposed: [UnsafeCell<Option<T>>; 2],
    common2: AtomicUsize,
    n: usize,
}

const FIRST: usize = 0;

impl<T> Common2Consensus<T> {
    pub fn new(n: usize) -> Self {
        assert!(n <= 2);

        Self {
            proposed: [UnsafeCell::new(None), UnsafeCell::new(None)],
            common2: AtomicUsize::new(FIRST),
            n,
        }
    }
}

impl<T> Default for Common2Consensus<T> {
    fn default() -> Self {
        Self::new(2)
    }
}

impl<T: Clone> Consensus<T> for Common2Consensus<T> {
    fn decide(&self, value: T, id: usize) -> T {
        assert!(id < self.n);

        self.propose(value, id);

        if self.common2.fetch_add(1, Ordering::AcqRel) == FIRST {
            cell_read(&self.proposed[id]).unwrap()
        } else {
            cell_read(&self.proposed[1 - id]).unwrap()
        }
    }
}

impl<T: Clone> ConsensusProtocol<T> for Common2Consensus<T> {
    fn propose(&self, value: T, id: usize) {
        cell_write(&self.proposed[id], Some(value));
    }
}

unsafe impl<T: Send> Send for Common2Consensus<T> {}
unsafe impl<T: Sync> Sync for Common2Consensus<T> {}

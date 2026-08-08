use crate::cell::UnsafeCell;
use crate::sync::atomic::{AtomicUsize, Ordering};

use super::api::{Consensus, ConsensusProtocol, cell_read, cell_write};

pub struct CasConsensus<T> {
    proposed: Vec<UnsafeCell<Option<T>>>,
    cas: AtomicUsize,
}

const FIRST: usize = usize::MAX;

impl<T> CasConsensus<T> {
    pub fn new(n: usize) -> Self {
        Self {
            proposed: (0..n).map(|_| UnsafeCell::new(None)).collect(),
            cas: AtomicUsize::new(FIRST),
        }
    }
}

impl<T> Default for CasConsensus<T> {
    fn default() -> Self {
        Self::new(2)
    }
}

impl<T: Clone> Consensus<T> for CasConsensus<T> {
    fn decide(&self, value: T, id: usize) -> T {
        assert!(id < self.proposed.len());

        self.propose(value, id);

        if self
            .cas
            .compare_exchange(FIRST, id, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
        {
            cell_read(&self.proposed[id]).unwrap()
        } else {
            cell_read(&self.proposed[self.cas.load(Ordering::Acquire)]).unwrap()
        }
    }
}

impl<T: Clone> ConsensusProtocol<T> for CasConsensus<T> {
    fn propose(&self, value: T, id: usize) {
        cell_write(&self.proposed[id], Some(value));
    }
}

unsafe impl<T: Send> Send for CasConsensus<T> {}
unsafe impl<T: Sync> Sync for CasConsensus<T> {}

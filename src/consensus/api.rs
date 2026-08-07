use crate::cell::UnsafeCell;

#[cfg(not(feature = "check-loom"))]
pub(crate) fn cell_read<T: Clone>(cell: &UnsafeCell<T>) -> T {
    unsafe { (*cell.get()).clone() }
}

#[cfg(feature = "check-loom")]
pub(crate) fn cell_read<T: Clone>(cell: &UnsafeCell<T>) -> T {
    unsafe { (*cell.get().deref()).clone() }
}

#[cfg(not(feature = "check-loom"))]
pub(crate) fn cell_write<T>(cell: &UnsafeCell<T>, value: T) {
    unsafe { *cell.get() = value }
}

#[cfg(feature = "check-loom")]
pub(crate) fn cell_write<T>(cell: &UnsafeCell<T>, value: T) {
    unsafe { *cell.get_mut().deref() = value }
}

pub trait Consensus<T>: Default {
    fn decide(&self, value: T, id: usize) -> T;
}

pub(crate) trait ConsensusProtocol<T>: Consensus<T> {
    fn propose(&self, value: T, id: usize);
}

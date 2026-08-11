pub mod consensus;
pub mod lock;
pub mod universal;

#[cfg(not(feature = "check-loom"))]
pub use std::*;

#[cfg(feature = "check-loom")]
pub use loom::*;

pub fn model<F: Fn() + Sync + Send + 'static>(f: F) {
    cfg_if::cfg_if! {
        if #[cfg(feature = "check-loom")] {
            loom::model(f)
        } else {
            f()
        }
    }
}

use crate::cell::UnsafeCell;

#[cfg(not(feature = "check-loom"))]
pub(crate) fn cell_read<T>(cell: &UnsafeCell<T>) -> &T {
    unsafe { &*cell.get() }
}

#[allow(clippy::mut_from_ref)]
#[cfg(not(feature = "check-loom"))]
pub(crate) fn cell_read_mut<T>(cell: &UnsafeCell<T>) -> &mut T {
    unsafe { &mut *cell.get() }
}

#[cfg(feature = "check-loom")]
pub(crate) fn cell_read<T>(cell: &UnsafeCell<T>) -> &T {
    unsafe { cell.with(|p| &*p) }
}

#[allow(clippy::mut_from_ref)]
#[cfg(feature = "check-loom")]
pub(crate) fn cell_read_mut<T>(cell: &UnsafeCell<T>) -> &mut T {
    unsafe { cell.with_mut(|p| &mut *p) }
}

#[cfg(not(feature = "check-loom"))]
pub(crate) fn cell_write<T>(cell: &UnsafeCell<T>, value: T) {
    unsafe { *cell.get() = value }
}

#[cfg(feature = "check-loom")]
pub(crate) fn cell_write<T>(cell: &UnsafeCell<T>, value: T) {
    unsafe { *cell.get_mut().deref() = value }
}

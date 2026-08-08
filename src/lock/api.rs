#![allow(dead_code)]
use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};

pub trait BoundedRawLock: Send + Sync {
    fn new(n: usize) -> Self;
    fn lock(&self, id: usize);
    /// # Safety
    ///
    /// id 必须是调用 lock 时传入的 id
    unsafe fn unlock(&self, id: usize);
}

pub struct BoundedLock<L: BoundedRawLock, T> {
    inner: L,
    data: UnsafeCell<T>,
}

impl<L: BoundedRawLock, T> BoundedLock<L, T> {
    pub fn new(n: usize, data: T) -> Self {
        Self {
            inner: L::new(n),
            data: UnsafeCell::new(data),
        }
    }
    pub fn lock(&self, id: usize) -> LockGuard<'_, L, T> {
        self.inner.lock(id);
        LockGuard { lock: self, id }
    }
}

unsafe impl<L: BoundedRawLock, T: Send> Send for BoundedLock<L, T> {}
unsafe impl<L: BoundedRawLock, T: Send> Sync for BoundedLock<L, T> {}

pub struct LockGuard<'a, L: BoundedRawLock, T> {
    lock: &'a BoundedLock<L, T>,
    id: usize,
}

impl<L: BoundedRawLock, T> Deref for LockGuard<'_, L, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<L: BoundedRawLock, T> DerefMut for LockGuard<'_, L, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<L: BoundedRawLock, T> Drop for LockGuard<'_, L, T> {
    fn drop(&mut self) {
        unsafe {
            self.lock.inner.unlock(self.id);
        }
    }
}

unsafe impl<L: BoundedRawLock, T: Send> Send for LockGuard<'_, L, T> {}
unsafe impl<L: BoundedRawLock, T: Send> Sync for LockGuard<'_, L, T> {}

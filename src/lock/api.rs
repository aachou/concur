use std::ops::{Deref, DerefMut};

use crate::cell::UnsafeCell;
use crate::{cell_read, cell_read_mut};

pub trait BoundedRawLock: Send + Sync {
    /// n 是支持的最大线程数量，线程 id 的范围为 0 ~ n-1。
    fn new(n: usize) -> Self;
    /// # Safety
    ///
    /// 一个线程 id 只能调用 lock 一次，释放锁后才能重新使用这个 id。
    unsafe fn lock(&self, id: usize);
    /// # Safety
    ///
    /// id 必须与调用 lock 时传入的 id 一致。
    unsafe fn unlock(&self, id: usize);
}

pub struct BoundedLock<L: BoundedRawLock, T> {
    inner: L,
    data: UnsafeCell<T>,
}

impl<L: BoundedRawLock, T> BoundedLock<L, T> {
    /// n 是支持的最大线程数量，线程 id 的范围为 0 ~ n-1。
    pub fn new(n: usize, data: T) -> Self {
        Self {
            inner: L::new(n),
            data: UnsafeCell::new(data),
        }
    }
    /// # Safety
    ///
    /// 一个线程 id 只能调用 lock 一次，释放锁后才能重新使用这个 id。
    pub unsafe fn lock(&self, id: usize) -> BoundedLockGuard<'_, L, T> {
        unsafe { self.inner.lock(id) };
        BoundedLockGuard { lock: self, id }
    }
}

unsafe impl<L: BoundedRawLock, T: Send> Send for BoundedLock<L, T> {}
unsafe impl<L: BoundedRawLock, T: Send> Sync for BoundedLock<L, T> {}

pub struct BoundedLockGuard<'a, L: BoundedRawLock, T> {
    lock: &'a BoundedLock<L, T>,
    id: usize,
}

impl<L: BoundedRawLock, T> Deref for BoundedLockGuard<'_, L, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        cell_read(&self.lock.data)
    }
}

impl<L: BoundedRawLock, T> DerefMut for BoundedLockGuard<'_, L, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        cell_read_mut(&self.lock.data)
    }
}

impl<L: BoundedRawLock, T> Drop for BoundedLockGuard<'_, L, T> {
    fn drop(&mut self) {
        unsafe {
            self.lock.inner.unlock(self.id);
        }
    }
}

unsafe impl<L: BoundedRawLock, T: Send> Send for BoundedLockGuard<'_, L, T> {}
unsafe impl<L: BoundedRawLock, T: Sync> Sync for BoundedLockGuard<'_, L, T> {}

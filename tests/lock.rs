#[cfg(not(feature = "check-loom"))]
mod basic {
    use relaxed_memory_concurrency::lock::{
        BakeryLock, BoundedLock, BoundedRawLock, FilterLock, PetersonLock,
    };
    use relaxed_memory_concurrency::thread;

    fn test_lock<L: BoundedRawLock>() {
        let lock = BoundedLock::<L, usize>::new(20, 0);

        thread::scope(|s| {
            let lock = &lock;

            (0..20).for_each(|id| {
                s.spawn(move || {
                    (0..500).for_each(|_| {
                        *lock.lock(id) += 1;
                    });
                });
            });
        });

        assert_eq!(*lock.lock(0), 10000);
    }

    #[test]
    fn peterson_lock() {
        let lock = BoundedLock::<PetersonLock, usize>::new(2, 0);

        thread::scope(|s| {
            let lock = &lock;

            (0..2).for_each(|id| {
                s.spawn(move || {
                    (0..500).for_each(|_| {
                        *lock.lock(id) += 1;
                    });
                });
            });
        });

        assert_eq!(*lock.lock(0), 1000);
    }

    #[test]
    fn filter_lock() {
        test_lock::<FilterLock>();
    }

    #[test]
    fn bakery_lock() {
        test_lock::<BakeryLock>();
    }
}

mod correctness {
    use relaxed_memory_concurrency::lock::{BakeryLock, BoundedRawLock, FilterLock, PetersonLock};
    use relaxed_memory_concurrency::sync::Arc;
    use relaxed_memory_concurrency::sync::atomic::{AtomicUsize, Ordering};
    use relaxed_memory_concurrency::thread;

    fn test_lock<L: BoundedRawLock + 'static>() {
        relaxed_memory_concurrency::model(|| {
            let lock = Arc::new(L::new(2));
            let data = Arc::new(AtomicUsize::new(0));

            let (lock_, data_) = (Arc::clone(&lock), Arc::clone(&data));
            let t1 = thread::spawn(move || {
                lock_.lock(0);
                data_.store(data_.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
                unsafe {
                    lock_.unlock(0);
                }
            });

            let (lock_, data_) = (Arc::clone(&lock), Arc::clone(&data));
            let t2 = thread::spawn(move || {
                lock_.lock(1);
                data_.store(data_.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
                unsafe {
                    lock_.unlock(1);
                }
            });

            t1.join().unwrap();
            t2.join().unwrap();

            assert_eq!(data.load(Ordering::Relaxed), 2);
        });
    }

    #[test]
    fn peterson_lock() {
        test_lock::<PetersonLock>();
    }

    #[test]
    fn filter_lock() {
        test_lock::<FilterLock>();
    }

    #[test]
    fn bakery_lock() {
        test_lock::<BakeryLock>();
    }
}

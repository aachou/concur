#[cfg(not(feature = "check-loom"))]
mod basic {
    use relaxed_memory_concurrency::lock::{
        BakeryLock, BoundedLock, BoundedRawLock, FilterLock, PetersonLock,
    };
    use relaxed_memory_concurrency::thread;

    fn test_lock<L: BoundedRawLock>(n: usize) {
        let lock = BoundedLock::<L, usize>::new(n, 0);

        thread::scope(|s| {
            let lock = &lock;

            (0..n).for_each(|id| {
                s.spawn(move || {
                    (0..500).for_each(|_| {
                        unsafe { *lock.lock(id) += 1 };
                    });
                });
            });
        });

        assert_eq!(unsafe { *lock.lock(0) }, 500 * n);
    }

    #[test]
    fn peterson_lock() {
        test_lock::<PetersonLock>(2);
    }

    #[test]
    fn filter_lock() {
        test_lock::<FilterLock>(20);
    }

    #[test]
    fn bakery_lock() {
        test_lock::<BakeryLock>(20);
    }
}

mod correctness {
    use relaxed_memory_concurrency::lock::{
        BakeryLock, BoundedLock, BoundedRawLock, FilterLock, PetersonLock,
    };
    use relaxed_memory_concurrency::sync::Arc;
    use relaxed_memory_concurrency::thread;

    fn test_lock<L: BoundedRawLock + 'static>(n: usize) {
        relaxed_memory_concurrency::model(move || {
            let lock = Arc::new(BoundedLock::<L, usize>::new(n, 0));

            let ts = (0..n)
                .map(|id| {
                    let lock = Arc::clone(&lock);
                    thread::spawn(move || unsafe {
                        *lock.lock(id) += 1;
                    })
                })
                .collect::<Vec<_>>();

            ts
                .into_iter()
                .for_each(|handle| handle.join().unwrap());

            assert_eq!(unsafe { *lock.lock(0) }, n)
        });
    }

    #[test]
    fn peterson_lock() {
        test_lock::<PetersonLock>(2);
    }

    #[test]
    fn filter_lock() {
        test_lock::<FilterLock>(2);
    }

    #[test]
    fn bakery_lock() {
        test_lock::<BakeryLock>(2);
    }
}

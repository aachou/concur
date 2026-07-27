#[cfg(not(feature = "check-loom"))]
mod basic {
    use relaxed_memory_concurrency::lock::{BakeryLock, FilterLock, PetersonLock};
    use relaxed_memory_concurrency::sync::atomic::{AtomicUsize, Ordering};
    use relaxed_memory_concurrency::thread;

    #[test]
    fn peterson_lock() {
        let peterson = PetersonLock::new();
        let data = AtomicUsize::new(0);

        thread::scope(|s| {
            let p = &peterson;
            let d = &data;

            for id in 0..2 {
                s.spawn(move || {
                    for _ in 0..500 {
                        p.lock(id);
                        d.store(d.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
                        p.unlock(id);
                    }
                });
            }
        });

        assert_eq!(data.load(Ordering::Relaxed), 1000);
    }

    #[test]
    fn filter_lock() {
        let filter = FilterLock::new(20);
        let data = AtomicUsize::new(0);

        thread::scope(|s| {
            let f = &filter;
            let d = &data;

            for id in 0..20 {
                s.spawn(move || {
                    for _ in 0..500 {
                        f.lock(id);
                        d.store(d.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
                        f.unlock(id);
                    }
                });
            }
        });

        assert_eq!(data.load(Ordering::Relaxed), 10000);
    }

    #[test]
    fn bakery_lock() {
        let bakery = BakeryLock::new(20);
        let data = AtomicUsize::new(0);

        thread::scope(|s| {
            let b = &bakery;
            let d = &data;

            for id in 0..20 {
                s.spawn(move || {
                    for _ in 0..500 {
                        b.lock(id);
                        d.store(d.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
                        b.unlock(id);
                    }
                });
            }
        });

        assert_eq!(data.load(Ordering::Relaxed), 10000);
    }
}

mod correctness {
    use relaxed_memory_concurrency::lock::{BakeryLock, FilterLock, PetersonLock};
    use relaxed_memory_concurrency::sync::Arc;
    use relaxed_memory_concurrency::sync::atomic::{AtomicUsize, Ordering};
    use relaxed_memory_concurrency::thread;

    #[test]
    fn peterson_lock() {
        relaxed_memory_concurrency::model(|| {
            let peterson = Arc::new(PetersonLock::new());
            let data = Arc::new(AtomicUsize::new(0));

            let p = Arc::clone(&peterson);
            let d = Arc::clone(&data);
            let t1 = thread::spawn(move || {
                p.lock(0);
                d.store(d.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
                p.unlock(0);
            });

            let p = Arc::clone(&peterson);
            let d = Arc::clone(&data);
            let t2 = thread::spawn(move || {
                p.lock(1);
                d.store(d.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
                p.unlock(1);
            });

            t1.join().unwrap();
            t2.join().unwrap();

            assert_eq!(data.load(Ordering::Relaxed), 2);
        });
    }

    #[test]
    fn filter_lock() {
        relaxed_memory_concurrency::model(|| {
            let filter = Arc::new(FilterLock::new(2));
            let data = Arc::new(AtomicUsize::new(0));

            let f = Arc::clone(&filter);
            let d = Arc::clone(&data);
            let t1 = thread::spawn(move || {
                f.lock(0);
                d.store(d.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
                f.unlock(0);
            });

            let f = Arc::clone(&filter);
            let d = Arc::clone(&data);
            let t2 = thread::spawn(move || {
                f.lock(1);
                d.store(d.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
                f.unlock(1);
            });

            t1.join().unwrap();
            t2.join().unwrap();

            assert_eq!(data.load(Ordering::Relaxed), 2);
        });
    }

    #[test]
    fn bakery_lock() {
        relaxed_memory_concurrency::model(|| {
            let bakery = Arc::new(BakeryLock::new(2));
            let data = Arc::new(AtomicUsize::new(0));

            let b = Arc::clone(&bakery);
            let d = Arc::clone(&data);
            let t1 = thread::spawn(move || {
                b.lock(0);
                d.store(d.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
                b.unlock(0);
            });

            let b = Arc::clone(&bakery);
            let d = Arc::clone(&data);
            let t2 = thread::spawn(move || {
                b.lock(1);
                d.store(d.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
                b.unlock(1);
            });

            t1.join().unwrap();
            t2.join().unwrap();

            assert_eq!(data.load(Ordering::Relaxed), 2);
        });
    }
}

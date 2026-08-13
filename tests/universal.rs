use relaxed_memory_concurrency::universal::SeqObject;

struct Stack<T> {
    inner: Vec<T>,
}

impl<T> Stack<T> {
    fn new() -> Self {
        Self { inner: Vec::new() }
    }
    fn push(&mut self, item: T) {
        self.inner.push(item);
    }
    fn pop(&mut self) -> Option<T> {
        self.inner.pop()
    }
}

impl<T> Default for Stack<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SeqObject for Stack<T> {}

#[cfg(not(feature = "check-loom"))]
mod basic {
    use super::Stack;
    use relaxed_memory_concurrency::thread;
    use relaxed_memory_concurrency::universal::{LFUniversal, WFUniversal};

    #[test]
    fn lf_universal() {
        let lfu = LFUniversal::<Stack<usize>>::new(20);

        thread::scope(|s| {
            let lfu = &lfu;

            (0..20).for_each(|id| {
                s.spawn(move || {
                    for item in 0..500 {
                        unsafe { lfu.apply(move |s: &mut Stack<usize>| s.push(item), id) }
                        unsafe {
                            assert!(lfu.apply(move |s: &mut Stack<usize>| s.pop(), id).is_some());
                        }
                    }
                });
            })
        })
    }

    #[test]
    fn wf_universal() {
        let wfu = WFUniversal::<Stack<usize>>::new(20);

        thread::scope(|s| {
            let wfu = &wfu;

            (0..20).for_each(|id| {
                s.spawn(move || {
                    for item in 0..500 {
                        unsafe { wfu.apply(move |s: &mut Stack<usize>| s.push(item), id) }
                        unsafe {
                            assert!(wfu.apply(move |s: &mut Stack<usize>| s.pop(), id).is_some());
                        }
                    }
                });
            })
        })
    }
}

mod correctness {
    use super::Stack;
    use relaxed_memory_concurrency::sync::Arc;
    use relaxed_memory_concurrency::thread;
    use relaxed_memory_concurrency::universal::{LFUniversal, WFUniversal};

    #[test]
    fn lf_universal() {
        relaxed_memory_concurrency::model(|| {
            let lfu = Arc::new(LFUniversal::<Stack<usize>>::new(2));

            let f = Arc::clone(&lfu);
            let t1 = thread::spawn(move || unsafe {
                f.apply(move |s: &mut Stack<usize>| s.push(0), 0);
            });

            let f = Arc::clone(&lfu);
            let t2 = thread::spawn(move || unsafe {
                if let Some(item) = f.apply(move |s: &mut Stack<usize>| s.pop(), 1) {
                    assert_eq!(item, 0);
                }
            });

            t1.join().unwrap();
            t2.join().unwrap();
        });
    }

    #[test]
    fn wf_universal() {
        relaxed_memory_concurrency::model(|| {
            let wfu = Arc::new(WFUniversal::<Stack<usize>>::new(2));

            let w = Arc::clone(&wfu);
            let t1 = thread::spawn(move || unsafe {
                w.apply(move |s: &mut Stack<usize>| s.push(0), 0);
            });

            let w = Arc::clone(&wfu);
            let t2 = thread::spawn(move || unsafe {
                if let Some(item) = w.apply(move |s: &mut Stack<usize>| s.pop(), 1) {
                    assert_eq!(item, 0);
                }
            });

            t1.join().unwrap();
            t2.join().unwrap();
        });
    }
}

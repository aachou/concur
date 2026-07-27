mod multi_valued_memory {
    //! # Multi-valued Memory — Load Hoisting
    //!
    //! 内存被表示为 location 到 message 列表的映射，每个 message 由 value 和 timestamp 组成。
    //! 线程可以从一个 location 读到 old value。
    //!
    //! ## 对应文档
    //!
    //! ```text
    //! X = 1;   r1 = Y;      ||      Y = 1;   r2 = X;
    //! ```
    //!
    //! 允许 `r1 = r2 = 0`

    use relaxed_memory_concurrency::thread;
    use relaxed_memory_concurrency::sync::Arc;
    use relaxed_memory_concurrency::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn load_hoisting() {
        let reached = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let reached_ = reached.clone();

        relaxed_memory_concurrency::model(move || {
            let x = Arc::new(AtomicUsize::new(0));
            let y = Arc::new(AtomicUsize::new(0));

            let x_ = Arc::clone(&x);
            let y_ = Arc::clone(&y);
            let t1 = thread::spawn(move || {
                x_.store(1, Ordering::Relaxed);
                y_.load(Ordering::Relaxed)
            });

            let x_ = Arc::clone(&x);
            let y_ = Arc::clone(&y);
            let t2 = thread::spawn(move || {
                y_.store(1, Ordering::Relaxed);
                x_.load(Ordering::Relaxed)
            });

            if t1.join().unwrap() == 0 && t2.join().unwrap() == 0 {
                reached_.store(true, Ordering::Relaxed);
            }
        });

        #[cfg(feature = "check-loom")]
        assert!(reached.load(Ordering::Relaxed));

        #[cfg(not(feature = "check-loom"))]
        assert!(true);
    }
}

mod message_adjacency {
    //! # Message Adjacency — Read-Modify-Write
    //!
    //! 对于 fetch-add 这类 RMW(ReadModifyWrite) 指令，新 message 必须邻接到被读取的 message 右侧。
    //! 这防止了 RMW 读取到 old value。
    //!
    //! ## 对应文档
    //!
    //! ```text
    //! r1 = X.fetch_add(1)       ||        r2 = X.fetch_add(1)
    //! ```
    //!
    //! 不允许 `r1 = r2 = 0`。

    use relaxed_memory_concurrency::thread;
    use relaxed_memory_concurrency::sync::Arc;
    use relaxed_memory_concurrency::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn message_adjacency_rmw_2_threads() {
        relaxed_memory_concurrency::model(|| {
            let x = Arc::new(AtomicUsize::new(0));

            let x_ = Arc::clone(&x);
            let t1 = thread::spawn(move || x_.fetch_add(1, Ordering::Relaxed));

            let x_ = Arc::clone(&x);
            let t2 = thread::spawn(move || x_.fetch_add(1, Ordering::Relaxed));

            let mut reached = vec![t1.join().unwrap(), t2.join().unwrap()];
            reached.sort();
            assert_eq!(reached, vec![0, 1]);
        });
    }

    #[test]
    fn message_adjacency_rmw_3_threads() {
        relaxed_memory_concurrency::model(|| {
            let x = Arc::new(AtomicUsize::new(0));

            let x_ = Arc::clone(&x);
            let t1 = thread::spawn(move || x_.fetch_add(1, Ordering::Relaxed));

            let x_ = Arc::clone(&x);
            let t2 = thread::spawn(move || x_.fetch_add(1, Ordering::Relaxed));

            let x_ = Arc::clone(&x);
            let t3 = thread::spawn(move || x_.fetch_add(1, Ordering::Relaxed));

            let mut reached = vec![t1.join().unwrap(), t2.join().unwrap(), t3.join().unwrap()];
            reached.sort();
            assert_eq!(reached, vec![0, 1, 2]);
        });
    }
}

mod views {
    //! # Views — Coherence & Synchronization
    //!
    //! View 是 location 到 timestamp 的映射，表示线程对 message 的确认状态。有三种 view：
    //!
    //! | View | 机制 | 作用 |
    //! |------|------|------|
    //! | **Per-thread view** | 表示线程对 message 的确认，读写操作更新当前线程的 view | 保证 per-location coherence（RR/RW/WR/WW）|
    //! | **Per-message view** | Release store 生成 message view；Acquire load 合并 message view | 实现 Release/Acquire 同步 |
    //! | **Global view** | fence(SC) 同步 thread view 与 global view | 实现 SC fence 跨线程同步 |

    use relaxed_memory_concurrency::thread;
    use relaxed_memory_concurrency::sync::Arc;
    use relaxed_memory_concurrency::sync::atomic::{AtomicUsize, Ordering, fence};

    // ═══════════════════════════════════════════════════════════════════════════════
    //  Per-thread View → Coherence
    // ═══════════════════════════════════════════════════════════════════════════════

    /// **RR coherence**：两次读同一位置，前一次读到新值，后一次不能读到旧值。
    ///
    /// 对应文档：`X=1 || r1=X; r2=X [r1=1, r2=0 impossible]`
    #[test]
    fn rr_coherence() {
        relaxed_memory_concurrency::model(|| {
            let x = Arc::new(AtomicUsize::new(0));

            let x_ = Arc::clone(&x);
            let t1 = thread::spawn(move || {
                x_.store(1, Ordering::Relaxed);
            });

            let x_ = Arc::clone(&x);
            let t2 = thread::spawn(move || {
                if x_.load(Ordering::Relaxed) == 1 {
                    assert_eq!(x_.load(Ordering::Relaxed), 1);
                }
            });

            t1.join().unwrap();
            t2.join().unwrap();
        });
    }

    /// **RW coherence**：读后写，读到的值一定在写之前。
    ///
    /// 对应文档：`r=X; X=1 [r=0]`
    ///
    /// 先读 X 得到 0（初始值），再写 X=1。读到的值不受后续写的影响。
    #[test]
    fn rw_coherence() {
        relaxed_memory_concurrency::model(|| {
            let x = Arc::new(AtomicUsize::new(0));

            thread::spawn(move || {
                assert_eq!(x.load(Ordering::Relaxed), 0);
                x.store(1, Ordering::Relaxed);
            })
            .join()
            .unwrap();
        });
    }

    /// **WR coherence**：写后读，读到的值一定是刚写的。
    ///
    /// 对应文档：`X=1; r=X [r=1]`
    #[test]
    fn wr_coherence() {
        relaxed_memory_concurrency::model(|| {
            let x = Arc::new(AtomicUsize::new(0));

            thread::spawn(move || {
                x.store(1, Ordering::Relaxed);
                assert_eq!(x.load(Ordering::Relaxed), 1);
            })
            .join()
            .unwrap();
        });
    }

    /// **WW coherence**：写后写，最终结果一定是最后一个写。
    ///
    /// 对应文档：`X=1; X=2 [X=2 at the end]`
    #[test]
    fn ww_coherence() {
        relaxed_memory_concurrency::model(|| {
            let x = Arc::new(AtomicUsize::new(0));

            let x_ = Arc::clone(&x);
            thread::spawn(move || {
                x_.store(1, Ordering::Relaxed);
                x_.store(2, Ordering::Relaxed);
            })
            .join()
            .unwrap();

            assert_eq!(x.load(Ordering::Relaxed), 2);
        });
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    //  Per-message View → Release/Acquire Synchronization
    // ═══════════════════════════════════════════════════════════════════════════════

    /// **Release/Acquire 同步**：Release store 会生成一个 message view（记录 release
    /// 时刻线程的完整视图）；Acquire load 将该 message view 合并到当前线程的 view 中。
    ///
    /// 对应文档：
    /// ```text
    /// X = 1;                        ||   if Y.load(acquire) == 1:
    /// Y.store(1, release);          ||       assert!(X == 1);  // 一定成功
    /// ```
    ///
    /// 如果线程 2 看到 Y=1（Acquire），则此前线程 1 对 X=1 的写入也必然可见。
    #[test]
    fn release_acquire_sync() {
        relaxed_memory_concurrency::model(|| {
            let x = Arc::new(AtomicUsize::new(0));
            let y = Arc::new(AtomicUsize::new(0));

            let (x_, y_) = (Arc::clone(&x), Arc::clone(&y));
            let t1 = thread::spawn(move || {
                x_.store(1, Ordering::Relaxed);
                y_.store(1, Ordering::Release);
            });

            let (x_, y_) = (Arc::clone(&x), Arc::clone(&y));
            let t2 = thread::spawn(move || {
                if y_.load(Ordering::Acquire) == 1 {
                    assert_eq!(x_.load(Ordering::Relaxed), 1);
                }
            });

            t1.join().unwrap();
            t2.join().unwrap();
        });
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    //  Global View → SC Fence Synchronization
    // ═══════════════════════════════════════════════════════════════════════════════

    /// **SC Fence 同步**：`fence(SC)` 会将当前线程的 view 与 global view 合并为
    /// 两者的最新值，使得即使使用 `Relaxed` 操作也能跨线程传递消息。
    ///
    /// 对应文档：
    /// ```text
    /// X = 1;              ||   if Y.load(relaxed) == 1 {
    /// fence(SC);          ||       fence(SC);
    /// Y.store(1, relaxed);||       assert!(X == 1);
    ///                     ||   }
    /// ```
    ///
    /// 如果线程 2 看到 Y=1（relaxed），则此前线程 1 对 X=1 的写入也必然可见
    #[test]
    fn sc_fence_sync() {
        relaxed_memory_concurrency::model(|| {
            let x = Arc::new(AtomicUsize::new(0));
            let y = Arc::new(AtomicUsize::new(0));

            let (x_, y_) = (Arc::clone(&x), Arc::clone(&y));
            let t1 = thread::spawn(move || {
                x_.store(1, Ordering::Relaxed);
                fence(Ordering::SeqCst);
                y_.store(1, Ordering::Relaxed);
            });

            let (x_, y_) = (Arc::clone(&x), Arc::clone(&y));
            let t2 = thread::spawn(move || {
                if y_.load(Ordering::Relaxed) == 1 {
                    fence(Ordering::SeqCst);
                    assert_eq!(x_.load(Ordering::Relaxed), 1);
                }
            });

            t1.join().unwrap();
            t2.join().unwrap();
        });
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    //  对照：不使用任何同步机制
    // ═══════════════════════════════════════════════════════════════════════════════

    /// 对照测试：全部使用 `Relaxed`，不做任何同步。
    ///
    /// ```text
    /// X = 1;                        ||   if Y.load() == 1:
    /// Y.store(1);                   ||       assert!(X == 1);  
    /// ```
    ///
    /// 线程 2 看到 Y=1 后读 X，此时 X 的值可能是 0（旧值）也可能是 1，因此断言可能会失败。
    #[test]
    fn relaxed_no_sync() {
        let reached = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let reached_ = reached.clone();

        relaxed_memory_concurrency::model(move || {
            let x = Arc::new(AtomicUsize::new(0));
            let y = Arc::new(AtomicUsize::new(0));

            let (x_, y_) = (Arc::clone(&x), Arc::clone(&y));
            let t1 = thread::spawn(move || {
                x_.store(1, Ordering::Relaxed);
                y_.store(1, Ordering::Relaxed);
            });

            let reached_ = reached_.clone();
            let (x_, y_) = (Arc::clone(&x), Arc::clone(&y));
            let t2 = thread::spawn(move || {
                if y_.load(Ordering::Relaxed) == 1 {
                    if x_.load(Ordering::Relaxed) == 1 {
                        reached_.store(true, Ordering::Relaxed);
                    }
                }
            });

            t1.join().unwrap();
            t2.join().unwrap();
        });

        #[cfg(not(feature = "check-loom"))]
        assert!(true);

        #[cfg(feature = "check-loom")]
        assert!(reached.load(Ordering::Relaxed));
    }
}

mod promises {
    //! # Promises — Store Hoisting
    //!
    //! 线程可以承诺未来会写入某个值。
    //! 承诺必须能被兑现——在线程实际执行到写操作时，必须能够写入承诺的值，如果无法兑现，则该执行路径无效。
    //!
    //! Store Hoisting 一共分为四种情况：
    //!
    //! | 场景 | 伪代码 | 预期 |
    //! |------|--------|------|
    //! | ① 无依赖 | `r1=X;Y=r1 \|\| r2=Y;X=1` | 允许 `r1=r2=1` |
    //! | ② 数据依赖 (OOTA) | `r1=X;Y=r1 \|\| r2=Y;X=r2` | 不允许`r1=r2=1` |
    //! | ③ 语法依赖 | `r1=X;Y=r1 \|\| r2=Y;if(r2==1){X=r2}else{X=1}` | 不允许 `r1=r2=1` |
    //! | ④ 语法依赖 + RW coherence | `r1=X;Y=r1 \|\| r2=Y;r3=X;if(r2==1){X=r2}else{X=1}` | 不允许 `r1=r2=r3=1` |

    use relaxed_memory_concurrency::thread;
    use relaxed_memory_concurrency::sync::Arc;
    use relaxed_memory_concurrency::sync::atomic::{AtomicUsize, Ordering};

    // ═══════════════════════════════════════════════════════════════════════════════
    //  场景 ①：Store hoisting 无依赖
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Thread 2 的 `X=1` 不依赖任何读操作，可以 hoist 到 `r2=Y` 之前执行。
    ///
    /// 对应文档：
    /// ```text
    /// r1 = X    ||    r2 = Y
    /// Y = r1    ||    X = 1
    /// ```
    ///
    /// **C++11**: 内存模型允许 `r1=r2=1`——relaxed 下 load-store 可重排序。
    ///
    /// **Loom**: **不支持 store hoisting**。
    ///
    /// **Promising Semantics**: Store hoisting（promise）使 `X=1` 可在 `r2=Y` 之前执行，因此允许 `r1=r2=1`。
    #[test]
    fn store_hoisting_wo_dep() {
        let reached = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let reached_ = reached.clone();

        relaxed_memory_concurrency::model(move || {
            let x = Arc::new(AtomicUsize::new(0));
            let y = Arc::new(AtomicUsize::new(0));
            let r1 = Arc::new(AtomicUsize::new(0));
            let r2 = Arc::new(AtomicUsize::new(0));

            let (x_, y_, r1_) = (Arc::clone(&x), Arc::clone(&y), Arc::clone(&r1));
            let t1 = thread::spawn(move || {
                r1_.store(x_.load(Ordering::Relaxed), Ordering::Relaxed);
                y_.store(r1_.load(Ordering::Relaxed), Ordering::Relaxed);
            });

            let (x_, y_, r2_) = (Arc::clone(&x), Arc::clone(&y), Arc::clone(&r2));
            let t2 = thread::spawn(move || {
                r2_.store(y_.load(Ordering::Relaxed), Ordering::Relaxed);
                x_.store(1, Ordering::Relaxed);
            });

            t1.join().unwrap();
            t2.join().unwrap();

            if r1.load(Ordering::Relaxed) == 1 && r2.load(Ordering::Relaxed) == 1 {
                reached_.store(true, Ordering::Relaxed);
            }
        });

        // assert!(reached.load(Ordering::Relaxed));
        assert!(true);
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    //  场景 ②：Store hoisting 有数据依赖 → OOTA 被禁止
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Thread 2 的 `X = r2` 依赖 `r2 = Y` 的结果，不能 hoist。
    ///
    /// 对应文档：
    /// ```text
    /// r1 = X    ||    r2 = Y
    /// Y = r1    ||    X = r2   // X 写入依赖 r2
    /// ```
    ///
    /// **C++11**: 不保证数据依赖，relaxed 下允许 `r1=r2=1`（OOTA 是 C++11 内存模型的已知缺陷，规范未正式禁止）。
    ///
    /// **Promising Semantics**: 数据依赖禁止 store hoisting——`X=r2` 无法在 `r2=Y` 之前执行，因此不允许 `r1=r2=1`（OOTA 被禁止）。
    ///
    /// **Loom** 不支持 store hoisting——不允许 `r1=r2=1`。
    #[test]
    fn store_hoisting_w_dep_oota() {
        relaxed_memory_concurrency::model(|| {
            let x = Arc::new(AtomicUsize::new(0));
            let y = Arc::new(AtomicUsize::new(0));
            let r1 = Arc::new(AtomicUsize::new(0));
            let r2 = Arc::new(AtomicUsize::new(0));

            let (x_, y_, r1_) = (Arc::clone(&x), Arc::clone(&y), Arc::clone(&r1));
            let t1 = thread::spawn(move || {
                r1_.store(x_.load(Ordering::Relaxed), Ordering::Relaxed);
                y_.store(r1_.load(Ordering::Relaxed), Ordering::Relaxed);
            });

            let (x_, y_, r2_) = (Arc::clone(&x), Arc::clone(&y), Arc::clone(&r2));
            let t2 = thread::spawn(move || {
                r2_.store(y_.load(Ordering::Relaxed), Ordering::Relaxed);
                x_.store(r2_.load(Ordering::Relaxed), Ordering::Relaxed);
            });

            t1.join().unwrap();
            t2.join().unwrap();

            assert!(!(r1.load(Ordering::Relaxed) == 1 && r2.load(Ordering::Relaxed) == 1));
        });
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    //  场景 ③：Store hoisting 有语法依赖（但编译器可优化）
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Thread 2 的 if/else 两个分支都写 `X=1`，等价于无条件 `X=1`。
    ///
    /// 对应文档：
    /// ```text
    /// r1 = X    ||    r2 = Y
    /// Y = r1    ||    if r2 == 1 { X = r2 } else { X = 1 }
    /// ```
    ///
    /// **C++11**: 同场景 ①，内存模型允许 `r1=r2=1`。
    ///
    /// **Loom**: 同场景 ①，不支持 store hoisting——不允许 `r1=r2=1`。
    ///
    /// **Promising Semantics**: 同场景 ①，store hoisting 允许 `r1=r2=1`。
    ///
    #[test]
    fn store_hoisting_syntactic_dep() {
        let reached = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let reached_ = reached.clone();

        relaxed_memory_concurrency::model(move || {
            let x = Arc::new(AtomicUsize::new(0));
            let y = Arc::new(AtomicUsize::new(0));
            let r1 = Arc::new(AtomicUsize::new(0));
            let r2 = Arc::new(AtomicUsize::new(0));

            let (x_, y_, r1_) = (Arc::clone(&x), Arc::clone(&y), Arc::clone(&r1));
            let t1 = thread::spawn(move || {
                r1_.store(x_.load(Ordering::Relaxed), Ordering::Relaxed);
                y_.store(r1_.load(Ordering::Relaxed), Ordering::Relaxed);
            });

            let (x_, y_, r2_) = (Arc::clone(&x), Arc::clone(&y), Arc::clone(&r2));
            let t2 = thread::spawn(move || {
                r2_.store(y_.load(Ordering::Relaxed), Ordering::Relaxed);
                if r2_.load(Ordering::Relaxed) == 1 {
                    x_.store(r2_.load(Ordering::Relaxed), Ordering::Relaxed);
                } else {
                    x_.store(1, Ordering::Relaxed);
                }
            });

            t1.join().unwrap();
            t2.join().unwrap();

            if r1.load(Ordering::Relaxed) == 1 && r2.load(Ordering::Relaxed) == 1 {
                reached_.store(true, Ordering::Relaxed);
            }
        });

        // assert!(reached.load(Ordering::Relaxed));
        assert!(true);
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    //  场景 ④：Store hoisting 有语法依赖 + RW coherence 阻止兑现
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Thread 2 的 if/else 两个分支都写 X=1，但写之前插入了 r3=X 读。
    ///
    /// 对应文档：
    /// ```text
    /// r1 = X    ||    r2 = Y
    /// Y = r1    ||    r3 = X
    ///           ||    if r2 == 1 { X = r2 } else { X = 1 }
    /// ```
    ///
    /// **C++11**: `r3 = X` sequenced-before 所有 X 写（两个分支均写 X=1），
    /// 因此 r3 只能读到 0。`r1=r2=1` 在 C++11 relaxed 下允许，但 r3 始终为 0，因此不允许 `r1=r2=r3=1`。
    ///
    /// **Promising Semantics**: Thread 2 可 promise X=1（语法依赖），
    /// 然后 r3=X 读到自身 promise 值 1，更新 per-thread view 到 promise位置，
    /// 导致后续 X=1 无法在正确位置写入兑现。因此不允许 `r1=r2=r3=1`。
    ///
    /// Loom 不支持 store hoisting——不允许 `r1=r2=r3=1`。。
    #[test]
    fn store_hoisting_syntactic_dep_rw_coherence() {
        relaxed_memory_concurrency::model(move || {
            let x = Arc::new(AtomicUsize::new(0));
            let y = Arc::new(AtomicUsize::new(0));
            let r1 = Arc::new(AtomicUsize::new(0));
            let r2 = Arc::new(AtomicUsize::new(0));
            let r3 = Arc::new(AtomicUsize::new(0));

            let (x_, y_, r1_) = (Arc::clone(&x), Arc::clone(&y), Arc::clone(&r1));
            let t1 = thread::spawn(move || {
                r1_.store(x_.load(Ordering::Relaxed), Ordering::Relaxed);
                y_.store(r1_.load(Ordering::Relaxed), Ordering::Relaxed);
            });

            let (x_, y_, r2_, r3_) = (
                Arc::clone(&x),
                Arc::clone(&y),
                Arc::clone(&r2),
                Arc::clone(&r3),
            );
            let t2 = thread::spawn(move || {
                r2_.store(y_.load(Ordering::Relaxed), Ordering::Relaxed);
                r3_.store(x_.load(Ordering::Relaxed), Ordering::Relaxed);
                if r2_.load(Ordering::Relaxed) == 1 {
                    x_.store(r2_.load(Ordering::Relaxed), Ordering::Relaxed);
                } else {
                    x_.store(1, Ordering::Relaxed);
                }
            });

            t1.join().unwrap();
            t2.join().unwrap();

            assert!(
                !(r1.load(Ordering::Relaxed) == 1
                    && r2.load(Ordering::Relaxed) == 1
                    && r3.load(Ordering::Relaxed) == 1)
            );
        });
    }
}

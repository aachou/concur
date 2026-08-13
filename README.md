# Relaxed Memory Concurrency

> 使用 [Loom](https://github.com/tokio-rs/loom) 对 Relaxed Behaviors & Orderings 以及一些并发数据结构进行测试，还有一些并发编程的学习文档。

## 1. Relaxed Behaviors & Orderings

### 1.1 Multi-Valued Memory — Load Hoisting

内存被表示为 location → message list 的映射，线程可以读到旧值。

```
X = 1;   r1 = Y;     ||     Y = 1;   r2 = X;
```

| 测试 | 验证 | 预期 |
|------|------|------|
| `load_hoisting` | Relaxed 下 load 可读到旧值 | 允许 `r1=r2=0` |

### 1.2 Message Adjacency — RMW Atomicity

RMW 操作的新 message 必须邻接到被读 message 的右侧，防止 RMW 读到旧值。

| 测试 | 验证 | 预期 |
|------|------|------|
| `message_adjacency_rmw_2_threads` | 双线程 `fetch_add(1)` | 不会同时读到 0 |
| `message_adjacency_rmw_3_threads` | 三线程 `fetch_add(1)` | 每个线程读到唯一值，最终 X=3 |

### 1.3 Views — Coherence & Synchronization

三种 View 约束线程行为：

- **Per-thread view** → 4 种 per-location coherence
- **Per-message view** → Release/Acquire 同步
- **Global view** → SC fence 同步

| 测试 | 对应机制 | 验证 |
|------|---------|------|
| `rr_coherence` | RR | `X=1;r1=X;r2=X` => 如果 `r1=1` 则 `r2!=0` |
| `rw_coherence` | RW | `r=X;X=1` => `r=0` |
| `wr_coherence` | WR | `X=1;r=X` => `r=1` |
| `ww_coherence` | WW | `X=1;X=2` => 最终 `X=2` |
| `release_acquire_sync` | Per-message View | Release/Acquire 保证消息传递 |
| `sc_fence_sync` | Global View | 双 SC fence 保证同步 |
| `relaxed_no_sync` | 对照 | 无同步时读旧值合法 |

### 1.4 Promises — Store Hoisting

线程可承诺未来写入某个值，承诺必须能被兑现。

Store hoisting (`r1=X;Y=r1 || r2=Y;X=1 → r1=r2=1`) 在 C++11 内存模型下允许。Loom 不支持 store hoisting。Promising Semantics 通过 promise 机制显式建模 store hoisting。

| 测试 | 场景 | C++11 | Loom | PS |
|------|------|-------|------|----|
| `store_hoisting_wo_dep` | 无依赖 | 允许 | 不支持 | 允许 |
| `store_hoisting_w_dep_oota` | 数据依赖 (OOTA) | 允许（已知缺陷） | 不支持 | 不允许 |
| `store_hoisting_syntactic_dep` | 语法依赖 | 允许 | 不支持 | 允许 |
| `store_hoisting_syntactic_dep_rw_coherence` | 语法依赖 + RW coherence | 不允许 | 不支持 | 不允许 |

## 2. 自旋锁

| Lock | 算法 | 最大线程数 | 同步模式 |
|------|------|-----------|---------|
| `PetersonLock` | Peterson 算法（flag + victim） | 2 | SC fence + Release/Acquire |
| `FilterLock` | Peterson 推广（N-1 层过滤） | N | SC fence + Release/Acquire |
| `BakeryLock` | Bakery 算法（取号排队） | N | SC fence + Release/Acquire |

## 3. 共识协议

| 共识对象 | 算法 | 最大线程数 | 同步模式 |
|---------|------|-----------|---------|
| `QueueConsensus` | FIFO 队列（先出队者获胜） | 2 | Mutex lock |
| `Common2Consensus` | common2（先执行 common2 RMW 者获胜） | 2 | Release/Acquire |
| `MultiConsensus` | 多重赋值（打锦标赛，最先赋值者获胜） | N | Mutex lock |
| `CasConsensus` | compare & swap（执行 cas 成功者获胜） | ∞ | Release/Acquire |

## 4. 并发对象通用构造

| 通用构造 | 算法 | 最大线程数 | 同步模式 |
|---------|------|-----------|---------|
| `LFUniversal` | 无锁通用构造（重放日志） | ∞ | Release/Acquire |
| `WFUniversal` | 无等待通用构造（帮助机制） | ∞ | Release/Acquire |

## 5. 运行

```powershell
cargo test          # 基本测试
cargo loom-test     # 使用 loom 进行测试
```

运行测试，Loom 会穷举所有线程交错和重排序，验证断言在所有调度下均成立。

## 6. 推荐学习资料

- [KAIST CS431: Concurrent Programming](https://github.com/kaist-cp/cs431)
- [多处理器编程的艺术](./docs/multiprocessor-programming-chp-01.md)
- [线性一致性](./docs/herlihy-wing-linearizability-summary.md)
- [分布式系统中的时间、时钟以及事件顺序](./docs/lamport-time-clocks-summary.md)
- [宽松内存并发入门](./relaxed%20memory%20concurrency.md)
- [关于共享内存一致性模型的教程](./docs/shared-memory-consistency-models-tutorial.md)
- [宽松内存并发同步模式](./docs/synchronization-patterns-zh.md)
- [crossbeam-epoch 内存序的使用](./docs/crossbeam-relaxed-memory-zh.md)
- [无锁哈希表](./docs/lock-free-hash-tables.md)
- [风险指针](./docs/hazard-pointers.md)
- [基于行为导向的并发](./docs/boc.md)

## 7. 参考

- [Promising Semantics](https://sf.snu.ac.kr/promise-concurrency/)
- [KAIST CS431: Concurrent Programming](https://github.com/kaist-cp/cs431)
- [Loom](https://github.com/tokio-rs/loom)
- [多处理器编程的艺术](https://www.cmpedu.com/books/book/5605553.htm)
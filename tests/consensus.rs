#[cfg(not(feature = "check-loom"))]
mod basic {
    use relaxed_memory_concurrency::consensus::{CasConsensus, Consensus, MultiConsensus};
    use relaxed_memory_concurrency::thread;

    fn test_consensus<C: Consensus<usize> + Sync>(c: C) {
        let succ = thread::scope(|s| {
            let c = &c;
            let proposes = (0..20)
                .map(|id| s.spawn(move || unsafe { *c.decide(id, id) }))
                .map(|handle| handle.join().unwrap())
                .collect::<Vec<usize>>();
            proposes.iter().all(|&p| p == proposes[0])
        });

        assert!(succ);
    }

    #[test]
    fn multi_consensus() {
        test_consensus(MultiConsensus::new(20));
    }

    #[test]
    fn cas_consensus() {
        test_consensus(CasConsensus::new(20));
    }
}

mod correctness {
    use relaxed_memory_concurrency::consensus::{
        CasConsensus, Common2Consensus, Consensus, MultiConsensus, QueueConsensus,
    };
    use relaxed_memory_concurrency::sync::Arc;
    use relaxed_memory_concurrency::thread;

    fn test_consensus<C: Consensus<usize> + Send + Sync + 'static>() {
        relaxed_memory_concurrency::model(|| {
            let c = Arc::new(C::default());

            let c_ = Arc::clone(&c);
            let t1 = thread::spawn(move || unsafe { *c_.decide(0, 0) });

            let c_ = Arc::clone(&c);
            let t2 = thread::spawn(move || unsafe { *c_.decide(1, 1) });

            assert_eq!(t1.join().unwrap(), t2.join().unwrap());
        })
    }

    #[test]
    fn queue_consensus() {
        test_consensus::<QueueConsensus<usize>>();
    }

    #[test]
    fn multi_consensus() {
        test_consensus::<MultiConsensus<usize>>();
    }

    #[test]
    fn common2_consensus() {
        test_consensus::<Common2Consensus<usize>>();
    }

    #[test]
    fn cas_consensus() {
        test_consensus::<CasConsensus<usize>>();
    }
}

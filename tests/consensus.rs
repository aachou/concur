#[cfg(not(feature = "check-loom"))]
mod basic {
    use relaxed_memory_concurrency::consensus::{Consensus, ConsensusObj, MultiConsensus};
    use relaxed_memory_concurrency::thread;

    fn test_consensus<C: Consensus<usize>>() {
        let c = ConsensusObj::<C, usize>::new(20);

        let succ = thread::scope(|s| {
            let c = &c;
            let mut t = Vec::with_capacity(20);
            for id in 0..20 {
                let ti = s.spawn(move || c.decide(id, id));
                t.push(ti);
            }
            let proposes = t
                .into_iter()
                .map(|item| item.join().unwrap())
                .collect::<Vec<usize>>();
            let mut succ = true;
            for id in 1..20 {
                if proposes[0] != proposes[id] {
                    succ = false;
                    break;
                }
            }
            succ
        });

        assert!(succ);
    }

    #[test]
    fn multi_consensus() {
        test_consensus::<MultiConsensus<usize>>();
    }
}

mod correctness {
    use relaxed_memory_concurrency::consensus::{
        Consensus, ConsensusObj, MultiConsensus, QueueConsensus,
    };
    use relaxed_memory_concurrency::sync::Arc;
    use relaxed_memory_concurrency::thread;

    fn test_consensus<C: Consensus<usize> + 'static>() {
        relaxed_memory_concurrency::model(|| {
            let c = Arc::new(ConsensusObj::<C, usize>::new(2));

            let c_ = Arc::clone(&c);
            let t1 = thread::spawn(move || c_.decide(0, 0));

            let c_ = Arc::clone(&c);
            let t2 = thread::spawn(move || c_.decide(1, 1));

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
}

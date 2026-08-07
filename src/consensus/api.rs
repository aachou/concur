use core::marker::PhantomData;

pub trait Consensus<T> {
    fn new(n: usize) -> Self;
    fn decide(&self, value: T, id: usize) -> T;
}

pub trait ConsensusProtocol<T>: Consensus<T> {
    fn propose(&self, value: T, id: usize);
}

pub struct ConsensusObj<C, T>
where
    C: Consensus<T>,
{
    inner: C,
    phantom: PhantomData<T>,
}

impl<C, T> Consensus<T> for ConsensusObj<C, T>
where
    C: Consensus<T>,
{
    fn new(n: usize) -> Self {
        Self {
            inner: C::new(n),
            phantom: PhantomData,
        }
    }

    fn decide(&self, value: T, id: usize) -> T {
        self.inner.decide(value, id)
    }
}

unsafe impl<C, T> Send for ConsensusObj<C, T> where C: Consensus<T> {}
unsafe impl<C, T> Sync for ConsensusObj<C, T> where C: Consensus<T> {}

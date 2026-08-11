pub trait Consensus<T>: Default {
    /// # Safety
    ///
    /// 一个线程 id 只能调用一次。
    unsafe fn decide(&self, value: T, id: usize) -> &T;
}

pub(crate) trait ConsensusProtocol<T>: Consensus<T> {
    fn propose(&self, value: T, id: usize);
}

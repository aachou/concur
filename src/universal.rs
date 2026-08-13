#![allow(unused, dead_code)]

use std::any::Any;
use std::marker::PhantomData;
use std::mem;
use std::ptr;

use crate::consensus::{CasConsensus, Consensus};
use crate::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

pub trait SeqObject: Default {
    fn apply<F, R>(&mut self, invoc: F) -> R
    where
        F: Fn(&mut Self) -> R,
    {
        invoc(self)
    }
}

type Invoc<T> = Box<dyn Fn(&mut T) -> Box<dyn Any>>;

struct Node<T> {
    invoc: Option<Invoc<T>>,
    decide_next: CasConsensus<*mut Node<T>>,
    seq: AtomicUsize,
    next: AtomicPtr<Node<T>>,
}

impl<T> Node<T> {
    fn new(invoc: Option<Invoc<T>>, n: usize) -> Self {
        Self {
            invoc,
            decide_next: CasConsensus::new(n),
            seq: AtomicUsize::new(1),
            next: AtomicPtr::new(ptr::null_mut()),
        }
    }
    fn max(array: &[AtomicPtr<Node<T>>]) -> *mut Node<T> {
        array
            .iter()
            .max_by(|x, y| unsafe {
                (*x.load(Ordering::Acquire))
                    .seq
                    .load(Ordering::Relaxed)
                    .cmp(&(*y.load(Ordering::Acquire)).seq.load(Ordering::Relaxed))
            })
            .unwrap()
            .load(Ordering::Acquire)
    }
}

pub struct LFUniversal<T> {
    head: Vec<AtomicPtr<Node<T>>>,
    tail: *mut Node<T>,
    phantom: PhantomData<Box<Node<T>>>,
}

impl<T> LFUniversal<T> {
    pub fn new(n: usize) -> Self {
        let node = Box::into_raw(Box::new(Node::<T>::new(None, n)));

        Self {
            head: (0..n).map(|_| AtomicPtr::new(node)).collect(),
            tail: node,
            phantom: PhantomData,
        }
    }

    /// # Safety
    ///
    /// 每个线程只能传自己的 id。
    pub unsafe fn apply<F, R>(&self, invoc: F, id: usize) -> R
    where
        F: Fn(&mut T) -> R + 'static,
        T: SeqObject,
        R: 'static,
    {
        assert!(id < self.head.len());

        let invoc = Box::new(move |s: &mut T| Box::new(invoc(s)) as Box<dyn Any>);
        let prefer = Box::into_raw(Box::new(Node::new(Some(invoc), self.head.len())));
        unsafe {
            while (*prefer).seq.load(Ordering::Relaxed) == 1 {
                let before = Node::max(&self.head);
                let after = *(*before).decide_next.decide(prefer, id);
                (*before).next.store(after, Ordering::Relaxed);
                (*after)
                    .seq
                    .store((*before).seq.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
                self.head[id].store(after, Ordering::Release);
            }
            let mut seq_object = T::default();
            let mut current = (*self.tail).next.load(Ordering::Relaxed);
            while current != prefer {
                let invoc = |s: &mut T| ((*current).invoc.as_ref().unwrap())(s);
                seq_object.apply(invoc);
                current = (*current).next.load(Ordering::Relaxed);
            }
            let invoc = |s: &mut T| {
                *((*current).invoc.as_ref().unwrap())(s)
                    .downcast::<R>()
                    .unwrap()
            };
            seq_object.apply(invoc)
        }
    }
}

impl<T> Drop for LFUniversal<T> {
    fn drop(&mut self) {
        let mut head = mem::take(&mut self.tail);
        while !head.is_null() {
            unsafe {
                let next = (*head).next.load(Ordering::Relaxed);
                drop(Box::from_raw(head));
                head = next;
            }
        }
    }
}

unsafe impl<T: Send> Send for LFUniversal<T> {}
unsafe impl<T: Sync> Sync for LFUniversal<T> {}

pub struct WFUniversal<T> {
    announce: Vec<AtomicPtr<Node<T>>>,
    head: Vec<AtomicPtr<Node<T>>>,
    tail: *mut Node<T>,
    phantom: PhantomData<Box<Node<T>>>,
}

impl<T> WFUniversal<T> {
    pub fn new(n: usize) -> Self {
        let node = Box::into_raw(Box::new(Node::<T>::new(None, n)));

        Self {
            announce: (0..n).map(|_| AtomicPtr::new(node)).collect(),
            head: (0..n).map(|_| AtomicPtr::new(node)).collect(),
            tail: node,
            phantom: PhantomData,
        }
    }

    /// # Safety
    ///
    /// 每个线程只能传自己的 id。
    pub unsafe fn apply<F, R>(&self, invoc: F, id: usize) -> R
    where
        F: Fn(&mut T) -> R + 'static,
        T: SeqObject,
        R: 'static,
    {
        assert!(id < self.head.len());

        let invoc = Box::new(move |s: &mut T| Box::new(invoc(s)) as Box<dyn Any>);
        let new = Box::into_raw(Box::new(Node::new(Some(invoc), self.head.len())));
        self.announce[id].store(new, Ordering::Release);
        self.head[id].store(Node::max(&self.head), Ordering::Release);
        unsafe {
            while (*new).seq.load(Ordering::Acquire) == 1 {
                let before = self.head[id].load(Ordering::Relaxed);
                let help = self.announce
                    [((*before).seq.load(Ordering::Relaxed) + 1) % self.head.len()]
                .load(Ordering::Acquire);
                let prefer = if help != self.tail && (*help).seq.load(Ordering::Relaxed) == 1 {
                    help
                } else {
                    new
                };
                let after = *(*before).decide_next.decide(prefer, id);
                (*before).next.store(after, Ordering::Relaxed);
                (*after)
                    .seq
                    .store((*before).seq.load(Ordering::Relaxed) + 1, Ordering::Release);
                self.head[id].store(after, Ordering::Release);
            }
            self.head[id].store(new, Ordering::Release);
            let mut seq_object = T::default();
            let mut current = (*self.tail).next.load(Ordering::Relaxed);
            while current != new {
                let invoc = |s: &mut T| ((*current).invoc.as_ref().unwrap())(s);
                seq_object.apply(invoc);
                current = (*current).next.load(Ordering::Relaxed);
            }
            let invoc = |s: &mut T| {
                *((*current).invoc.as_ref().unwrap())(s)
                    .downcast::<R>()
                    .unwrap()
            };
            seq_object.apply(invoc)
        }
    }
}

impl<T> Drop for WFUniversal<T> {
    fn drop(&mut self) {
        let mut head = mem::take(&mut self.tail);
        while !head.is_null() {
            unsafe {
                let next = (*head).next.load(Ordering::Relaxed);
                drop(Box::from_raw(head));
                head = next;
            }
        }
    }
}

unsafe impl<T: Send> Send for WFUniversal<T> {}
unsafe impl<T: Sync> Sync for WFUniversal<T> {}

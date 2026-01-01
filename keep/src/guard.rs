use crate::{
    alist::Node,
    heaped::Heap,
    tracked_atomic::{Mutation, TrackedAtomic},
};
use std::ops::Deref;


pub struct Guard<T>
{
    pub(crate) ptr: Heap<Mutation<T>>,
    pub(crate) node: Heap<Node<Mutation<T>>>,
    pub(crate) tracked_atomic: Heap<TrackedAtomic<T>>,
}


impl<T: std::fmt::Debug> std::fmt::Debug for Guard<T>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        self.ptr.borrow().fmt(f)
    }
}


impl<T: std::fmt::Display> std::fmt::Display for Guard<T>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        self.ptr.borrow().fmt(f)
    }
}


impl<T: PartialEq> PartialEq for Guard<T>
{
    fn eq(&self, other: &Self) -> bool
    {
        self.ptr.borrow().eq(other.ptr.borrow())
    }
}


impl<T> Deref for Guard<T>
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        unsafe { &*self.ptr.inner() }
    }
}


impl<T> AsRef<T> for Guard<T>
{
    fn as_ref(&self) -> &T
    {
        self
    }
}


impl<T> Clone for Guard<T>
{
    fn clone(&self) -> Self
    {
        Self {
            ptr: self.ptr,
            node: self.node.head().insert(self.ptr),
            tracked_atomic: self.tracked_atomic,
        }
    }
}


impl<T> Drop for Guard<T>
{
    fn drop(&mut self)
    {
        self.node.clear(self.ptr.as_ptr());
        self.tracked_atomic.try_drop(self.ptr);
    }
}

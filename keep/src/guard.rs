use crate::heap::{Heap, HeapRc};
use std::{marker::PhantomData, ptr::NonNull, sync::atomic::Ordering};


pub struct Guard<T>
{
    hrc: NonNull<HeapRc<T>>,
    _phantom: PhantomData<HeapRc<T>>,
}


impl<T> Guard<T>
{
    /// Creates a new `Guard<T>`
    pub fn new(val: impl Into<Self>) -> Self
    {
        val.into()
    }

    /// Returns true if `self` is guarding the same value as `other`
    pub fn compare(&self, other: &Self) -> bool
    {
        self.hrc == other.hrc
    }
}


impl<T: Into<HeapRc<T>>> From<T> for Guard<T>
{
    fn from(hrc: T) -> Self
    {
        Guard::from(Heap::from(hrc.into()).as_non_null())
    }
}


impl<T> From<Box<T>> for Guard<T>
where
    Box<T>: Into<HeapRc<T>>,
{
    fn from(hrc: Box<T>) -> Self
    {
        Guard::from(Heap::from(hrc.into()).as_non_null())
    }
}


impl<T> From<Heap<T>> for Guard<T>
where
    Heap<T>: Into<HeapRc<T>>,
{
    fn from(hrc: Heap<T>) -> Self
    {
        Guard::from(Heap::from(hrc.into()).as_non_null())
    }
}


impl<T> From<NonNull<HeapRc<T>>> for Guard<T>
{
    fn from(hrc: NonNull<HeapRc<T>>) -> Self
    {
        unsafe { hrc.as_ref().inc() };

        Self {
            hrc,
            _phantom: PhantomData,
        }
    }
}


impl<T> Clone for Guard<T>
{
    fn clone(&self) -> Self
    {
        Self::from(self.hrc)
    }
}


impl<T> std::ops::Deref for Guard<T>
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        unsafe { self.hrc.as_ref() }.data()
    }
}


impl<T> AsRef<T> for Guard<T>
{
    fn as_ref(&self) -> &T
    {
        &self
    }
}


impl<T> Drop for Guard<T>
{
    fn drop(&mut self)
    {
        if unsafe { self.hrc.as_ref().dec() } == 1
        {
            std::sync::atomic::fence(Ordering::Acquire);

            unsafe {
                let hrc = Box::from_raw(self.hrc.as_ptr());
                hrc.free_unchecked();
            }
        }
    }
}


impl<T: std::fmt::Debug> std::fmt::Debug for Guard<T>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        self.as_ref().fmt(f)
    }
}


// A Guard<T> is Send+Sync if T is Sync
unsafe impl<T: Sync> Send for Guard<T> {}
unsafe impl<T: Sync> Sync for Guard<T> {}

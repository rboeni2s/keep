use std::{
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};


/// Wraps a non null pointer to a heap allocated `T` that is never dropped.
pub struct Heap<T>(NonNull<T>);


impl<T> Heap<T>
{
    /// Creates a new Heap<T> from a pointer `ptr`
    ///
    /// # Safety
    /// * `ptr` must point to a heap allocated `T`
    /// * `ptr` must never be freed while a `Heap<T>` exists.
    pub unsafe fn from_ptr(ptr: *mut T) -> Self
    {
        Heap(unsafe { NonNull::new_unchecked(ptr) })
    }

    /// Consumes `Heap<T>` and frees `T`
    ///
    /// # Safety
    /// The specific `T` this `Heap<T>` was pointing to must never be accessed again after this call.
    pub unsafe fn free(self)
    {
        drop(unsafe { Box::from_raw(self.0.as_ptr()) });
    }

    /// Access the wrapped pointer
    pub fn as_non_null(&self) -> NonNull<T>
    {
        self.0
    }

    /// Creates a `Heap<T>`
    pub fn new(val: impl Into<Heap<T>>) -> Self
    {
        val.into()
    }
}


impl<T> From<T> for Heap<T>
{
    fn from(value: T) -> Self
    {
        Heap::from(Box::new(value))
    }
}


impl<T> From<Box<T>> for Heap<T>
{
    fn from(value: Box<T>) -> Self
    {
        // Box::into_raw is guaranteed to be not null
        Heap(unsafe { NonNull::new_unchecked(Box::into_raw(value)) })
    }
}


impl<T> std::ops::Deref for Heap<T>
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        unsafe { self.0.as_ref() }
    }
}


impl<T> Eq for Heap<T> {}
impl<T> PartialEq for Heap<T>
{
    fn eq(&self, other: &Self) -> bool
    {
        self.0 == other.0
    }
}


impl<T> Copy for Heap<T> {}
impl<T> Clone for Heap<T>
{
    fn clone(&self) -> Self
    {
        *self
    }
}


// A Heap<T> is Send+Sync if T is Sync
unsafe impl<T: Sync> Send for Heap<T> {}
unsafe impl<T: Sync> Sync for Heap<T> {}


/// Owns a heap allocated `T` while keeping track of a count of all of `T`'s accessors.
pub struct HeapRc<T>
{
    data: Heap<T>,
    ref_count: AtomicUsize,
}


impl<T> HeapRc<T>
{
    /// Creates a new `HeapRc<T>`
    pub fn new(val: impl Into<HeapRc<T>>) -> Self
    {
        val.into()
    }

    pub fn data(&self) -> &T
    {
        &self.data
    }

    /// Increments the reference count
    ///
    /// # Safety
    /// The number of calls ti `inc()` and `dec()` must add up eventually.
    pub unsafe fn inc(&self)
    {
        if self.ref_count.fetch_add(1, Ordering::Relaxed) >= isize::MAX as usize
        {
            std::process::abort();
        }
    }

    /// Decrements the reference count
    ///
    /// # Safety
    /// The number of calls ti `inc()` and `dec()` must add up eventually.
    pub unsafe fn dec(&self) -> usize
    {
        self.ref_count.fetch_sub(1, Ordering::Release)
    }

    /// Free its data and drops itself
    ///
    /// # Safety
    /// Do not free while there are references to self and/or the data
    pub unsafe fn free_unchecked(self)
    {
        unsafe { self.data.free() };
    }
}


impl<T: Into<Heap<T>>> From<T> for HeapRc<T>
{
    fn from(value: T) -> Self
    {
        HeapRc {
            data: value.into(),
            ref_count: AtomicUsize::new(0),
        }
    }
}


impl<T> From<Box<T>> for HeapRc<T>
where
    Box<T>: Into<Heap<T>>,
{
    fn from(value: Box<T>) -> Self
    {
        HeapRc {
            data: value.into(),
            ref_count: AtomicUsize::new(0),
        }
    }
}


impl<T> From<Heap<T>> for HeapRc<T>
{
    fn from(value: Heap<T>) -> Self
    {
        HeapRc {
            data: value,
            ref_count: AtomicUsize::new(0),
        }
    }
}


// A HeapRc<T> is Send+Sync if T is Sync
unsafe impl<T: Sync> Send for HeapRc<T> {}
unsafe impl<T: Sync> Sync for HeapRc<T> {}

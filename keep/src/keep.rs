use crate::{guard::Guard, heap::Heap};
use std::sync::atomic::{AtomicPtr, Ordering};


struct InnerKeep<T>
{
    mutation: AtomicPtr<Guard<T>>,
}


impl<T> InnerKeep<T>
{
    fn new(guard: Guard<T>) -> Self
    {
        let mutation = Heap::new(guard).as_non_null().as_ptr();

        Self {
            mutation: AtomicPtr::new(mutation),
        }
    }
}


impl<T> Drop for InnerKeep<T>
{
    fn drop(&mut self)
    {
        // Clear the mutation guard
        let mutation_ptr = self.mutation.swap(std::ptr::null_mut(), Ordering::AcqRel);

        // Free the old mutation guard if its not null
        if !mutation_ptr.is_null()
        {
            unsafe { Heap::from_ptr(mutation_ptr).free() };
        }
    }
}


pub struct Keep<T>
{
    inner: AtomicPtr<Guard<InnerKeep<T>>>,
}


impl<T> Keep<T>
{
    /// Creates a new `Keep<T>`
    pub fn new(val: impl Into<Keep<T>>) -> Self
    {
        val.into()
    }

    #[inline]
    fn load_inner(&self) -> &InnerKeep<T>
    {
        unsafe {
            self.inner
                .load(Ordering::Acquire)
                .as_ref()
                .expect("InnerKeep<T> was null")
        }
    }

    /// Read the current value of this keep.
    ///
    /// Returns a `Guard<T>` guarding the current value.
    pub fn read(&self) -> Guard<T>
    {
        unsafe { self.load_inner().mutation.load(Ordering::Acquire).as_ref() }
            .expect("Inner Guard was null")
            .clone()
    }

    /// Writes `val` to this keep.
    ///
    /// `val` becomes the new current value.
    pub fn write(&self, val: impl Into<Guard<T>>)
    {
        let new_guard = Heap::new(val.into()).as_non_null().as_ptr();
        let old_guard = self.load_inner().mutation.swap(new_guard, Ordering::AcqRel);

        // Drop the old guard
        unsafe { Heap::from_ptr(old_guard).free() };
    }

    /// Swaps the current value with `val` returning the old value
    pub fn swap(&self, val: impl Into<Guard<T>>) -> Guard<T>
    {
        let new_guard = Heap::new(val.into()).as_non_null().as_ptr();
        let old_guard = self.load_inner().mutation.swap(new_guard, Ordering::AcqRel);
        let old_guard = unsafe { Heap::from_ptr(old_guard) };

        // Backup a clone of the old guard
        let ret = (*old_guard).clone();

        // Drop the old guard
        unsafe { old_guard.free() };

        ret
    }

    /// Performs a compare and swap operation on the current value.
    pub fn cas(&self, current: &Guard<T>, new: impl Into<Guard<T>>) -> Result<Guard<T>, Guard<T>>
    {
        let new_guard = Heap::new(new.into()).as_non_null().as_ptr();

        let result =
            self.load_inner()
                .mutation
                .fetch_update(Ordering::Release, Ordering::Acquire, |curr| {
                    let curr = unsafe { curr.as_ref().expect("Current inner guard was null") };

                    if curr.compare(current)
                    {
                        Some(new_guard)
                    }
                    else
                    {
                        None
                    }
                });

        // Dereference *mut Guard<T> and clone them.
        // Also drop the old guard in case the result was ok
        result.map_err(|e| (unsafe { &*e }).clone()).map(|ok| {
            let old_guard = unsafe { Heap::from_ptr(ok) };
            let ret = (*old_guard).clone();
            unsafe { old_guard.free() };
            ret
        })
    }

    /// Swallows `other` making `self` point to `other`.
    ///
    /// Returns the old guard.
    pub fn swallow(&self, other: &Keep<T>) -> Guard<T>
    {
        // Get other and clone its guard
        let other_ptr = {
            let ptr = other.inner.load(Ordering::Acquire);
            let other = (unsafe { &*ptr }).clone();
            Heap::new(other).as_non_null().as_ptr()
        };

        // Swap out the "old" guard in self with other
        let old_ptr = self.inner.swap(other_ptr, Ordering::SeqCst);

        // Backup old guard mutation
        let ret = unsafe {
            (&*old_ptr)
                .mutation
                .load(Ordering::Acquire)
                .as_ref()
                .expect("Old mutation was null")
                .clone()
        };

        // Free the old guard
        unsafe { Heap::from_ptr(old_ptr).free() };
        ret
    }
}


impl<T> From<&Guard<T>> for Keep<T>
{
    fn from(value: &Guard<T>) -> Self
    {
        value.clone().into()
    }
}


impl<T, G: Into<Guard<T>>> From<G> for Keep<T>
{
    fn from(value: G) -> Self
    {
        let inner = Heap::new(Guard::new(InnerKeep::new(value.into())))
            .as_non_null()
            .as_ptr();

        Self {
            inner: AtomicPtr::new(inner),
        }
    }
}


impl<T> Drop for Keep<T>
{
    fn drop(&mut self)
    {
        let guard_ptr = self.inner.swap(std::ptr::null_mut(), Ordering::AcqRel);
        if !guard_ptr.is_null()
        {
            unsafe {
                Heap::from_ptr(guard_ptr).free();
            }
        }
    }
}


impl<T> Clone for Keep<T>
{
    fn clone(&self) -> Self
    {
        let guard = unsafe {
            self.inner
                .load(Ordering::Acquire)
                .as_ref()
                .expect("Trying to clone Keep<T> that contains null")
                .clone()
        };

        let inner = Heap::new(guard).as_non_null().as_ptr();

        Self {
            inner: AtomicPtr::new(inner),
        }
    }
}

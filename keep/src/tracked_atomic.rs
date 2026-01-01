use crate::{
    alist::Node,
    guard::Guard,
    heaped::{Heap, Heaped},
};
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};


pub struct Mutation<T>
{
    ptr: Heap<T>,
    freed: Heap<AtomicBool>, // This Flag will prevent double frees
}


impl<T> Mutation<T>
{
    pub fn inner(&self) -> *mut T
    {
        self.ptr.as_ptr()
    }

    pub fn borrow(&self) -> &T
    {
        &self.ptr
    }
}


impl<T> Mutation<T>
{
    fn new(ptr: impl Heaped<T>) -> Heap<Self>
    {
        unsafe {
            Self {
                ptr: ptr.heaped(),
                freed: AtomicBool::new(false).heaped(),
            }
            .heaped()
        }
    }
}


pub struct TrackedAtomic<T>
{
    accessor_count: AtomicUsize,
    mutation: AtomicPtr<Mutation<T>>,
    freed: Heap<Node<AtomicBool>>,
    domain: Heap<Node<Mutation<T>>>,
}


impl<T> TrackedAtomic<T>
{
    /// Creates a new tracked atomic initialized to `value`
    pub fn new(value: impl Heaped<T>) -> Heap<Self>
    {
        let mutation = Mutation::new(value);
        let head = Node::new(mutation, None);

        head.clear_unchecked();

        unsafe {
            Self {
                accessor_count: AtomicUsize::new(0),
                mutation: AtomicPtr::new(mutation.as_ptr()),
                domain: head,
                freed: Node::new(Heap::from_ptr(std::ptr::null_mut()), None),
            }
            .heaped()
        }
    }

    /// Registers a new accessor of this tracked atomic
    pub fn register_accessor(&self) -> usize
    {
        self.accessor_count.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Unregisters a accessor of this tacked atomic.
    ///
    /// Returns `true` if the last remaining accessor was just unregistered
    pub fn unregister_accessor(&self) -> bool
    {
        self.accessor_count.fetch_sub(1, Ordering::SeqCst) == 1
    }

    pub fn store(&self, new_value: impl Heaped<T>)
    {
        let new_value = Mutation::new(new_value);
        let old_value = self.mutation.swap(new_value.as_ptr(), Ordering::AcqRel);
        self.try_drop(unsafe { Heap::from_ptr(old_value) });
    }

    pub fn load(&self) -> Guard<T>
    {
        let ptr = unsafe { Heap::from_ptr(self.mutation.load(Ordering::Acquire)) };
        let node = self.domain.insert(ptr);

        Guard {
            ptr,
            node,
            // NOTE: This assumes that self is being stored on the heap.
            tracked_atomic: unsafe { Heap::from_ptr(self as *const _ as _) },
        }
    }

    pub fn swap(&self, new_value: impl Heaped<T>) -> Guard<T>
    {
        let new_value = Mutation::new(new_value);
        let old_value = self.mutation.swap(new_value.as_ptr(), Ordering::AcqRel);
        let old_value = unsafe { Heap::from_ptr(old_value) };

        Guard {
            ptr: old_value,
            node: self.domain.insert(old_value),
            tracked_atomic: unsafe { Heap::from_ptr(self as *const _ as _) },
        }
    }

    pub fn exchange(
        &self,
        current: &Guard<T>,
        new_value: impl Heaped<T>,
    ) -> Result<Guard<T>, Guard<T>>
    {
        let new_value = Mutation::new(new_value);
        let tracked_atomic = unsafe { Heap::from_ptr(self as *const _ as _) };

        self.mutation
            .compare_exchange(
                current.ptr.as_ptr(),
                new_value.as_ptr(),
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map(|old| {
                let old = unsafe { Heap::from_ptr(old) };

                Guard {
                    ptr: old,
                    node: self.domain.insert(old),
                    tracked_atomic,
                }
            })
            .map_err(|actual| {
                let actual = unsafe { Heap::from_ptr(actual) };

                Guard {
                    ptr: actual,
                    node: self.domain.insert(actual),
                    tracked_atomic,
                }
            })
    }

    fn drop_mutation(&self, mutation: &Mutation<T>) -> bool
    {
        if mutation
            .freed
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
        {
            unsafe { mutation.ptr.free() };
            self.freed.insert(mutation.freed);
            return true;
        }

        false
    }

    pub fn try_drop(&self, val: Heap<Mutation<T>>)
    {
        let accessors = self.accessor_count.load(Ordering::SeqCst);

        // If the value is part of the current mutation and still has accessors -> do not drop
        if self.mutation.load(Ordering::Acquire) == val.as_ptr() && accessors != 0
        {
            return;
        }

        if accessors == 0
        {
            // All Keeps are dead
            match self.domain.contains_or_empty(val.as_ptr())
            {
                Some(false) =>
                {
                    self.drop_mutation(&val);
                }

                None =>
                {
                    if self.drop_mutation(&val)
                    {
                        unsafe { self.destroy() };
                    }
                }

                _ => (),
            }
        }
        // Some Keep is still alive, so just try to free the value...
        else if !self.domain.contains(val.as_ptr())
        {
            self.drop_mutation(&val);
        }
    }

    pub fn is_dead(&self) -> bool
    {
        self.accessor_count.load(Ordering::SeqCst) == 0 && self.domain.is_all_empty()
    }

    pub unsafe fn destroy(&self)
    {
        // Free the mutation
        let mutation = self.mutation.load(Ordering::Acquire);
        unsafe {
            self.drop_mutation(&*mutation);
        }

        // Free the lists
        unsafe {
            self.domain.free_list();
            self.freed.free_list_and_nodes();
            self.domain.free();
            self.freed.free();
        }
    }
}

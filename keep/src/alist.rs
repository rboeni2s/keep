use crate::heaped::{Heap, Heaped};
use std::sync::atomic::{AtomicPtr, Ordering};


pub struct Node<T>
{
    head: Option<Heap<Node<T>>>,
    value: AtomicPtr<T>,
    next: AtomicPtr<Node<T>>,
}


impl<T> Node<T>
{
    /// Creates a new node without a next node
    pub fn new(value: impl Heaped<T>, head: Option<Heap<Node<T>>>) -> Heap<Node<T>>
    {
        unsafe {
            Self {
                head,
                value: AtomicPtr::new(value.heaped().as_ptr()),
                next: AtomicPtr::new(std::ptr::null_mut()),
            }
            .heaped()
        }
    }

    /// Frees the list
    pub unsafe fn free_list(&self)
    {
        if let Some(next) = unsafe {
            self.next
                .swap(std::ptr::null_mut(), Ordering::AcqRel)
                .as_ref()
        }
        {
            unsafe {
                next.free_list();
                next.heaped().free();
            }
        }
    }

    /// Frees the list and its values
    pub unsafe fn free_list_and_nodes(&self)
    {
        if let Some(next) = unsafe {
            self.next
                .swap(std::ptr::null_mut(), Ordering::AcqRel)
                .as_ref()
        }
        {
            unsafe {
                next.free_list();
                next.heaped().free();
            }
        }

        let val = self.value.load(Ordering::Acquire);
        if !val.is_null()
        {
            unsafe {
                Heap::from_ptr(val).free();
            }
        }
    }


    /// Finds a free node or appends a new one and then stores `new_val`.
    ///
    /// Returns a `Heap<Node<T>>` pointing to the node containing `new_val`.
    pub fn insert(&self, new_val: impl Heaped<T>) -> Heap<Node<T>>
    {
        let new_val = unsafe { new_val.heaped() };
        let current_val = self.value.load(Ordering::Acquire);

        // If the current value is null, try to use this node to store new_val
        if current_val.is_null()
            && self
                .value
                .compare_exchange(
                    current_val,
                    new_val.as_ptr(),
                    Ordering::Release,
                    Ordering::Relaxed,
                )
                .is_ok()
        {
            // NOTE: This assumes that self is on the heap!!!
            return unsafe { Heap::from_ptr(self as *const _ as _) };
        }

        // If this node is not clear, try to store new_val in the next node
        if let Some(next) = unsafe { self.next.load(Ordering::Acquire).as_ref() }
        {
            return next.insert(new_val);
        }

        // If there is no next node, create a new node and append it in the list
        let new_node = Node::<T>::new(new_val, Some(self.head()));

        match self.next.compare_exchange(
            std::ptr::null_mut(),
            new_node.as_ptr(),
            Ordering::Release,
            Ordering::Acquire,
        )
        {
            Ok(_) => new_node,

            // while we were working on this node, a next node appeared so try the insert on the next node again.
            Err(next) => unsafe { &*next }.insert(new_val),
        }
    }

    /// Clears the value of a node if it contains the `current` value
    pub fn clear(&self, current: *mut T) -> bool
    {
        self.value
            .compare_exchange(
                current,
                std::ptr::null_mut(),
                Ordering::Release,
                Ordering::Relaxed,
            )
            .is_ok()
    }

    /// Clears the value of a node
    pub fn clear_unchecked(&self)
    {
        self.value.store(std::ptr::null_mut(), Ordering::Release);
    }

    /// Returns `true` if this list contained a pointer `ptr`.
    pub fn contains(&self, ptr: *mut T) -> bool
    {
        if self.value.load(Ordering::Acquire) == ptr
        {
            return true;
        }

        if let Some(next) = unsafe { self.next.load(Ordering::Acquire).as_ref() }
        {
            return next.contains(ptr);
        }

        false
    }

    /// Returns `true` if this node and all child nodes are clear
    pub fn is_all_empty(&self) -> bool
    {
        if !self.value.load(Ordering::Acquire).is_null()
        {
            return false;
        }

        if let Some(next) = unsafe { self.next.load(Ordering::Acquire).as_ref() }
        {
            return next.is_all_empty();
        }

        true
    }

    /// Checks whether or not the list contains `ptr` while also checking if the list is empty.
    ///
    /// # Returns
    ///  * `None` if the list is empty
    ///  * `Some(false)` if the list is not empty but does not contain a `ptr`
    ///  * `Some(true)` if the list contains a `ptr`
    pub fn contains_or_empty(&self, ptr: *mut T) -> Option<bool>
    {
        let mut is_empty = true;
        let mut current = self as *const _ as *mut Node<T>;

        while let Some(curr) = unsafe { current.as_ref() }
        {
            if is_empty
            {
                is_empty = curr.value.load(Ordering::Acquire).is_null();
            }

            if !is_empty && curr.value.load(Ordering::Acquire) == ptr
            {
                return Some(true);
            }

            current = curr.next.load(Ordering::Acquire);
        }

        if is_empty
        {
            return None;
        }

        Some(false)
    }

    /// Returns the head of this list
    pub fn head(&self) -> Heap<Node<T>>
    {
        //NOTE: this assumes that self is on the heap
        self.head
            .unwrap_or(unsafe { Heap::from_ptr(self as *const _ as _) })
    }
}

use keep::*;
use std::sync::atomic::{AtomicUsize, Ordering};


/// A fixed size concurrent buffer
pub struct ConcurrentBuffer<T>
{
    last_index: AtomicUsize,
    capacity: usize,
    buffer: Box<[Keep<Option<Keep<T>>>]>,
}


impl<T> ConcurrentBuffer<T>
{
    /// Creates a new concurrent buffer with a capacity of `capacity`.
    pub fn with_capacity(capacity: usize) -> Self
    {
        let mut buf = Box::new_uninit_slice(capacity);

        for entry in &mut buf
        {
            entry.write(Keep::new(None));
        }

        let buf = unsafe { buf.assume_init() };

        Self {
            capacity,
            last_index: AtomicUsize::new(0),
            buffer: buf,
        }
    }

    /// Inserts an element `e` at position `index` into the buffer.
    ///
    /// # Returns
    /// * the old element as `Some(Keep<T>)` if a element was already present at `index`
    /// * `None` if no element was present at `index` or if the index was out of bounds.
    pub fn insert(&self, index: usize, e: impl Heaped<T>) -> Option<Keep<T>>
    {
        let keep = Keep::new(Some(Keep::new(e)));
        self.buffer.get(index)?.swap_with(&keep);

        if let Some(element) = &*keep.read()
        {
            return Some(element.clone());
        }

        None
    }

    /// Returns the element at position `index`
    ///
    /// # Returns
    /// * the element as `Some(Guard<T>)` if it exists at position `index`
    /// * `None` if the element does not exist or `index` is out of bounds
    pub fn get(&self, index: usize) -> Option<Guard<T>>
    {
        if let Some(element) = &*self.buffer.get(index)?.read()
        {
            return Some(element.read());
        }

        None
    }

    /// Tries to find a free slot and inserts `e` into it.
    ///
    /// # Returns
    /// * `Ok(index)` if the element was inserted successfully where `index` indicates the position of `e`
    /// * `Err(())` if the buffer has no free slot left
    #[allow(clippy::result_unit_err)] // I want the returned result to be an error if the buffer is full,
    //                                   because inserting without removing an old element failed.
    //                                   this error however has no value and that's why a unit err result is fine here.
    pub fn put(&self, e: impl Heaped<T>) -> Result<usize, ()>
    {
        let keep = Keep::new(Some(Keep::new(e)));
        let last_index = self.last_index.load(Ordering::Acquire);
        let (e, marker) = self.buffer[last_index].read_marked();

        // if the slot is free, try to insert into this slot
        if e.is_none() && self.buffer[last_index].swap_with_marked(marker, &keep)
        {
            // Swap worked! Advance last_index and return
            if last_index + 1 < self.capacity
            // only advance it if it stays in bounds
            {
                self.last_index.compare_exchange(
                    last_index,
                    last_index + 1,
                    Ordering::Release,
                    Ordering::Relaxed,
                );
            }

            return Ok(last_index);
        }

        // The slot is not free, search linearly for a free slot...
        for (i, slot) in self.buffer.iter().enumerate()
        {
            let (e, marker) = slot.read_marked();

            // if the slot is free, try to insert into this slot
            if e.is_none() && slot.swap_with_marked(marker, &keep)
            {
                // The swap worked, return the index of the new element
                return Ok(i);
            }
        }

        // No free slot was found, error out
        Err(())
    }
}

use keep::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};


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
    pub fn insert(&self, index: usize, e: impl Into<Guard<T>>) -> Option<Keep<T>>
    {
        let e = Some(Keep::new(e));
        let old = self.buffer.get(index)?.swap(e);

        if let Some(element) = &*old
        {
            return Some(element.clone());
        }

        None
    }

    /// Removes an element at position `index` from the buffer
    pub fn remove(&self, index: usize) -> Option<Keep<T>>
    {
        if self.buffer.get(index)?.read().is_some()
        {
            let old = self.buffer[index].swap(None);

            if let Some(value) = &*old
            {
                return Some(value.clone());
            }
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

    /// Tries to remove any element from the buffer
    pub fn pop(&self) -> Option<Keep<T>>
    {
        // Get the last index
        let last_index = self
            .last_index
            .fetch_update(Ordering::Release, Ordering::Acquire, |li| {
                Some(li.saturating_sub(1))
            })
            .unwrap()
            .saturating_sub(1);

        // Try to get the last index
        if let Some(element) = self.remove(last_index)
        {
            return Some(element);
        }

        // Fallback if the last_index missed:
        // Iterate over all slots
        for (i, slot) in self.buffer.iter().enumerate()
        {
            let e = slot.read();

            // if the slot is not free, try to take the slot
            if e.is_some()
                && let Ok(guard) = slot.cas(&e, None)
                {
                    self.set_index_hint(i); // Set the index hint to the slot that was just cleared
                    return guard.as_ref().clone();
                }
        }

        None
    }

    /// Tries to find a free slot and inserts `e` into it.
    ///
    /// # Returns
    /// * `Ok(index)` if the element was inserted successfully where `index` indicates the position of `e`
    /// * `Err(e)` if the buffer has no free slot left. E is the item passed as `e`.
    pub fn put(&self, e: impl Into<Guard<T>>) -> Result<usize, Keep<T>>
    {
        self.put_keep(Keep::new(e))
    }

    fn put_keep(&self, keep: Keep<T>) -> Result<usize, Keep<T>>
    {
        if self.capacity == 0
        {
            println!("PUT ON 0 CAP");
            return Err(keep);
        }

        let wrapped = Guard::new(Some(keep.clone()));

        // Get the last index
        let last_index = self
            .last_index
            .fetch_update(Ordering::Release, Ordering::Acquire, |li| {
                Some((li + 1).min(self.capacity.saturating_sub(1)))
            })
            .unwrap();

        // If the slot at last index is free, try to take it
        let maybe_free = self.buffer[last_index].read();
        if maybe_free.is_none()
            && self.buffer[last_index]
                .cas(&maybe_free, wrapped.clone())
                .is_ok()
            {
                return Ok(last_index);
            }

        // Fallback if last_index was not free:
        // The slot is not free, search linearly for a free slot...
        for (i, slot) in self.buffer.iter().enumerate()
        {
            let e = slot.read();

            // if the slot is free, try to insert into this slot
            if e.is_none() && slot.cas(&e, wrapped.clone()).is_ok()
            {
                // The swap worked, set last index and return the index of the new element
                self.set_index_hint(i + 1);
                return Ok(i);
            }
        }

        // No free slot was found, error out
        Err(keep)
    }

    /// Gives a hint to the buffer, that the next free index is `next_free`
    pub fn set_index_hint(&self, next_free: usize)
    {
        self.last_index.store(next_free, Ordering::Release);
    }
}


//TODO: Resizing does not work reliably in mt
pub struct RingBuffer<T>
{
    buffer: ConcurrentBuffer<T>,
    write_index: AtomicUsize,
    read_index: AtomicUsize,
}


impl<T> RingBuffer<T>
{
    const MIN_SIZE: usize = 32;

    /// Creates a new dynamic buffer.
    pub fn new() -> Self
    {
        Self::with_hint(Self::MIN_SIZE)
    }

    /// Create a `DynBuffer<T>` with a capacity of `hint`
    ///
    /// A hint of at least `Self::MIN_SIZE` will be enforced.
    pub fn with_hint(hint: usize) -> Self
    {
        Self {
            buffer: ConcurrentBuffer::with_capacity(hint.max(Self::MIN_SIZE)),
            write_index: AtomicUsize::new(0),
            read_index: AtomicUsize::new(0),
        }
    }

    /// Pushes a value `val` into the buffer
    pub fn push(&self, val: impl Into<Guard<T>>)
    {
        let index = self
            .write_index
            .fetch_update(Ordering::Release, Ordering::Acquire, |i| {
                Some((i + 1) % self.buffer.capacity)
            })
            .unwrap();

        self.buffer.insert(index, val);
    }


    /// Pops a value from the buffer
    pub fn pop(&self) -> Option<Keep<T>>
    {
        let write_index = self.write_index.load(Ordering::Acquire);
        let read_index = self
            .read_index
            .fetch_update(Ordering::Release, Ordering::Acquire, |i| {
                if i == write_index && self.buffer.get(i).is_none()
                {
                    None
                }
                else
                {
                    Some((i + 1) % self.buffer.capacity)
                }
            });

        if let Ok(read_index) = read_index
        {
            return self.buffer.remove(read_index);
        }

        None
    }

    /// Generates a snapshot of the buffer
    pub fn snapshot(&self) -> Snapshot<T>
    {
        Snapshot::from(&self.buffer)
    }

    /// Clears the buffer and returns it's contents
    pub fn flush(&self, max: Option<usize>) -> Vec<Keep<T>>
    {
        let max = max.unwrap_or(usize::MAX);
        let mut count = 0;
        let mut buffer = Vec::with_capacity(self.buffer.capacity);

        while let Some(item) = self.pop()
        {
            buffer.push(item);
            count += 1;

            if count >= max
            {
                break;
            }
        }

        buffer
    }
}


impl<T> Default for RingBuffer<T>
{
    fn default() -> Self
    {
        Self::new()
    }
}


/// Represents a snapshot of a concurrent buffer
pub struct Snapshot<T>
{
    st_buffer: Vec<Guard<T>>,
}


impl<T> AsMut<Vec<Guard<T>>> for Snapshot<T>
{
    fn as_mut(&mut self) -> &mut Vec<Guard<T>>
    {
        &mut self.st_buffer
    }
}


impl<T> AsRef<Vec<Guard<T>>> for Snapshot<T>
{
    fn as_ref(&self) -> &Vec<Guard<T>>
    {
        &self.st_buffer
    }
}


impl<T> From<&ConcurrentBuffer<T>> for Snapshot<T>
{
    fn from(value: &ConcurrentBuffer<T>) -> Self
    {
        let mut st_buffer = Vec::with_capacity(value.capacity);

        value
            .buffer
            .iter()
            .filter_map(|e| e.read().as_ref().as_ref().map(|g| g.read()))
            .collect_into(&mut st_buffer);

        Self { st_buffer }
    }
}

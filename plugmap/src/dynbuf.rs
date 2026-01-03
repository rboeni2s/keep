use keep::*;
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

    /// Removes an element at position `index` from the buffer
    pub fn remove(&self, index: usize) -> Option<Keep<T>>
    {
        if self.buffer.get(index)?.read().is_some()
        {
            let keep = Keep::new(None);
            self.buffer[index].swap_with(&keep);

            if let Some(value) = &*keep.read()
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
        let keep = Keep::new(None);

        // Iterate over all slots
        for (i, slot) in self.buffer.iter().enumerate()
        {
            let (e, marker) = slot.read_marked();

            // if the slot is not free, try to take the slot
            if e.is_some() && slot.swap_with_marked(marker, &keep)
            {
                return (*keep.read()).clone();
            }
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
        let last_index = self.last_index.fetch_add(1, Ordering::AcqRel);
        let (e, marker) = self.buffer.get(last_index).ok_or(())?.read_marked();

        // if the slot is free, try to insert into this slot
        if e.is_none() && self.buffer[last_index].swap_with_marked(marker, &keep)
        // not using get(index) is okay here, since i already know this index exists
        {
            // Swap worked!
            return Ok(last_index);
        }

        // The slot is not free, search linearly for a free slot...
        for (i, slot) in self.buffer.iter().enumerate()
        {
            let (e, marker) = slot.read_marked();

            // if the slot is free, try to insert into this slot
            if e.is_none() && slot.swap_with_marked(marker, &keep)
            {
                // The swap worked, set last index and return the index of the new element
                self.last_index.store(i + 1, Ordering::Release);
                return Ok(i);
            }
        }

        // No free slot was found, error out
        Err(())
    }

    /// Gives a hint to the buffer, that the next free index is `next_free`
    pub fn set_index_hint(&self, next_free: usize)
    {
        self.last_index.store(next_free, Ordering::Release);
    }
}


//TODO: Resizing does not work reliably in mt
pub struct DynBuffer<T>
{
    min_size: usize,
    buffer: Keep<ConcurrentBuffer<T>>,
    new_buffer: Keep<Option<Keep<ConcurrentBuffer<T>>>>,
    resizer: Keep<Option<Resizer<T>>>,
    count: AtomicUsize,
    readers: AtomicUsize,
}


impl<T> DynBuffer<T>
{
    const MIN_SIZE: usize = 4;

    /// Creates a new dynamic buffer.
    pub fn new() -> Self
    {
        Self::with_hint(Self::MIN_SIZE)
    }


    /// Create a `DynBuffer<T>` with a capacity of `hint^2`
    ///
    /// A hint of at least `Self::MIN_SIZE` will be enforced.
    pub fn with_hint(hint: usize) -> Self
    {
        Self {
            min_size: hint.max(Self::MIN_SIZE),
            buffer: Keep::new(ConcurrentBuffer::with_capacity(
                1 << hint.max(Self::MIN_SIZE),
            )),
            new_buffer: Keep::new(None),
            resizer: Keep::new(None),
            count: AtomicUsize::new(0),
            readers: AtomicUsize::new(0),
        }
    }

    /// Pushes a value `val` into the buffer
    pub fn push(&self, val: impl Heaped<T>)
    {
        // Help with resizing if a resize is ongoing...
        self.maybe_resize();

        let count = self.count.fetch_add(1, Ordering::AcqRel);

        self.consider_resize(count);
        {
            self.maybe_resize();
        }

        self.buffer.read().put(val);
    }

    /// Pops a value from the buffer
    pub fn pop(&self) -> Option<Keep<T>>
    {
        if self.count.load(Ordering::Acquire) == 0
        {
            return None;
        }

        // Help with ongoing resize
        self.maybe_resize();

        self.readers.fetch_add(1, Ordering::Release);

        let mut ret = self.buffer.read().pop();

        // If something was popped of, adjust size
        if ret.is_some()
        {
            self.count.fetch_sub(1, Ordering::Release);
        }

        self.readers.fetch_sub(1, Ordering::Release);

        ret
    }


    fn consider_resize(&self, index: usize) -> bool
    {
        let mut buf = None;
        let capacity = self.buffer.read().capacity;
        let new_buffer = self.new_buffer.read();

        if new_buffer.is_none()
        {
            // Do a resize up if the buffer is almost full
            if capacity <= index + 2
            {
                buf = Some(Keep::new(ConcurrentBuffer::with_capacity(capacity << 1)));
            }
            // Do a resize down if the buffer is less than half full
            else if (capacity >> 1) > index && capacity > (1 << self.min_size)
            {
                buf = Some(Keep::new(ConcurrentBuffer::with_capacity(capacity >> 1)));
            }

            // If a resize is needed, try to build a resizer
            if let Some(new) = &buf
            {
                let new = new.read();
                if self.new_buffer.exchange(&new_buffer, buf).is_ok()
                {
                    let curr = self.buffer.read();
                    let resizer = Resizer::new(8, curr, new);
                    self.resizer.write(Some(resizer));

                    return true;
                }
            }
        }

        self.new_buffer.read().is_some()
    }

    fn maybe_resize(&self)
    {
        // If a resize is in progress...
        if let Some(resizer) = &*self.resizer.read()
        {
            // wait for all pop operations to finish
            while self.readers.load(Ordering::Acquire) != 0
            {
                std::hint::spin_loop();
            }

            // ...help resizing
            resizer.resize();

            // Check if this thread needs to swap the buffers
            if resizer.do_swap()
            {
                // Swap the buffers and destroy the resizer
                if let Some(new_buffer) = &*self.new_buffer.swap(None)
                {
                    self.buffer.swap_with(new_buffer);
                    self.resizer.write(None);
                }
            }

            // spin until the entire resize is complete
            while self.resizer.read().is_some()
            {
                std::hint::spin_loop();
            }
        }
    }
}


impl<T> Default for DynBuffer<T>
{
    fn default() -> Self
    {
        Self::new()
    }
}


struct Resizer<T>
{
    current: Guard<ConcurrentBuffer<T>>,
    new: Guard<ConcurrentBuffer<T>>,
    length: usize,
    current_index: AtomicUsize,
    new_index: AtomicUsize,
    stride: usize,
    workers: AtomicUsize,
    swapped: AtomicBool,
}


impl<T> Resizer<T>
{
    fn new(
        stride: usize,
        current: Guard<ConcurrentBuffer<T>>,
        new: Guard<ConcurrentBuffer<T>>,
    ) -> Self
    {
        // Find the smallest capacity to determine the length of the resize
        let length = current.capacity.min(new.capacity);

        Self {
            current,
            new,
            length,
            stride,
            workers: AtomicUsize::new(0),
            current_index: AtomicUsize::new(0),
            new_index: AtomicUsize::new(0),
            swapped: AtomicBool::new(false),
        }
    }

    /// Returns `true` once, after that always `false`. Can be used to determine which
    /// thread is allowed to swap the old buffer with the resized one after the resize is complete...
    fn do_swap(&self) -> bool
    {
        !self.swapped.swap(true, Ordering::SeqCst)
    }

    /// Makes this thread help with the resize, will block until resize is complete
    fn resize(&self)
    {
        // increase worker counter by one
        self.workers.fetch_add(1, Ordering::Release);

        // Set the start and end bounds.
        let mut start = self.current_index.fetch_add(self.stride, Ordering::AcqRel);
        let mut end = self.current.capacity.min(start + self.stride);

        // Until start would be out of bounds and the smallest slot size of both buffers has not yet been reached...
        while start < end && self.new_index.load(Ordering::Acquire) < self.length
        {
            // ...read all elements of a stride in the current buffer...
            for entry in &self.current.buffer[start..end]
            {
                // ...and copy them into the new buffer if they are not empty.
                if let Some(old_entry) = &*entry.read()
                {
                    let new_index = self.new_index.fetch_add(1, Ordering::AcqRel);
                    let entry = Some(old_entry.clone());
                    self.new.buffer[new_index].write(entry);
                }
            }

            // Set the start and end bounds for the next stride
            start = self.current_index.fetch_add(self.stride, Ordering::AcqRel);
            end = self.current.capacity.min(start + self.stride);
        }

        // work is done, decrease worker count and spin until no workers remain
        let mut workers = self.workers.fetch_sub(1, Ordering::AcqRel) - 1;
        while workers != 0
        {
            workers = self.workers.load(Ordering::Acquire);
        }
    }
}

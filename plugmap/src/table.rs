use crate::{
    PlugMap,
    entry::{Entry, EntryNode},
};
use keep::*;
use std::sync::atomic::{AtomicUsize, Ordering};


pub struct Table<Key, Val>
{
    size: usize,
    capacity: usize,
    entry_count: AtomicUsize,
    entries: Box<[Keep<Entry<Key, Val>>]>,
}


impl<Key, Val> Table<Key, Val>
where
    Key: Eq,
{
    pub fn new(size: usize) -> Self
    {
        // assert that the table has at least 16 entries.
        let size = size.max(PlugMap::<Key, Val>::DEFAULT_SIZE);

        let mut entries = Box::new_uninit_slice(1 << size);

        for entry in &mut entries
        {
            entry.write(Keep::new(Entry::Empty));
        }

        Self {
            size,
            capacity: 1 << size,
            entry_count: AtomicUsize::new(0),
            entries: unsafe { entries.assume_init() },
        }
    }

    /// Creates a table with double the capacity
    #[inline]
    pub fn new_bigger(&self) -> Self
    {
        Self::new(self.size + 1)
    }

    #[inline]
    pub fn capacity(&self) -> usize
    {
        self.capacity
    }

    pub fn remove(&self, key: &Key, hash: u64) -> Option<Keep<Val>>
    {
        let entry = self.entry_of(hash);

        loop
        {
            let (entry_guard, marker) = entry.read_marked();

            match &*entry_guard
            {
                Entry::Empty => return None,

                Entry::Head(keep) =>
                {
                    let (entry_node, node_marker) = keep.read_marked();

                    if entry_node.key() == key
                    {
                        todo!()
                    }
                }
            }
        }
    }

    pub fn get(&self, key: &Key, hash: u64) -> Option<Guard<Val>>
    {
        self.entry_of(hash).read().search(key)
    }

    pub fn insert(&self, entry_node: EntryNode<Key, Val>) -> (Option<Keep<Val>>, bool)
    {
        let entry = self.entry_of(entry_node.hash());
        let entry_node = Keep::new(entry_node);

        loop
        {
            let (entry_guard, marker) = entry.read_marked();

            match &*entry_guard
            {
                Entry::Empty =>
                {
                    if entry.swap_with_marked(marker, &Keep::new(Entry::Head(entry_node.clone())))
                    {
                        let entry_count = self.entry_count.fetch_add(1, Ordering::SeqCst) + 1;
                        return (None, self.resize_needed_up(entry_count));
                    }
                }

                Entry::Head(keep) =>
                {
                    match keep.read().update(&entry_node)
                    {
                        Some(old) => return (Some(old), false),
                        None =>
                        {
                            let entry_count = self.entry_count.fetch_add(1, Ordering::SeqCst) + 1;
                            return (None, self.resize_needed_up(entry_count));
                        }
                    }
                }
            }
        }
    }

    /// Checks if the map needs to be resized up.
    ///
    /// This function assumes a power of two capacity greater than 2^2.
    #[inline]
    fn resize_needed_up(&self, entry_count: usize) -> bool
    {
        /*
        This checks if entry_count is at least 75% of self.capacity.
        the expression at the return position of this function can be read like this,
        assuming self.capacity is a power of two greater than 2^2

        let half_capacity = self.capacity >> 1;
        let quarter_capacity = self.capacity >> 2;
        let percent_75_capacity = half_capacity + quarter_capacity;
        entry_count >= percent_75_capacity
        */

        entry_count > (self.capacity >> 1) + (self.capacity >> 2)
    }

    #[inline]
    fn index_of(&self, hash: u64) -> usize
    {
        hash as usize & ((1 << self.size) - 1)
    }

    #[inline]
    fn entry_at(&self, index: usize) -> &Keep<Entry<Key, Val>>
    {
        &self.entries[index]
    }

    #[inline]
    fn entry_of(&self, hash: u64) -> &Keep<Entry<Key, Val>>
    {
        &self.entries[self.index_of(hash)]
    }

    #[inline]
    pub fn entries(&self) -> &[Keep<Entry<Key, Val>>]
    {
        &self.entries
    }
}


pub struct TableIter<Key, Val>
{
    pub(crate) table: Guard<Table<Key, Val>>,
    pub(crate) index: usize,
    pub(crate) bin_buffer: Vec<Guard<Val>>,
}


impl<Key, Val> Iterator for TableIter<Key, Val>
where
    Key: Eq,
{
    type Item = Guard<Val>;

    fn next(&mut self) -> Option<Self::Item>
    {
        loop
        {
            // get the next item from the bin buffer, if available
            if let Some(item) = self.bin_buffer.pop()
            {
                // return one item from the current bin buffer
                return Some(item);
            }

            // if no item is available in the current buffer:
            // load the next bin into the bin_buffer and advance the bin index,
            // return None if all bins have been visited.
            self.bin_buffer = self.table.entries.get(self.index)?.read().buffered();
            self.index += 1;
        }
    }
}

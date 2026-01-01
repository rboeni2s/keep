use crate::{entry::Entry, table::Table};
use keep::*;
use std::{
    hash::Hash,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};


pub struct Resizer<Key, Val>
{
    old_table: Guard<Table<Key, Val>>,
    new_table: Keep<Table<Key, Val>>,
    stride: usize,
    old_capacity: usize,
    index: AtomicUsize,
    workers: AtomicUsize,
    finished: AtomicBool,
}


impl<Key, Val> Resizer<Key, Val>
where
    Key: Eq,
{
    pub fn new(stride: usize, old_table: Guard<Table<Key, Val>>) -> Self
    {
        Self {
            new_table: Keep::new(old_table.new_bigger()),
            stride,
            old_capacity: old_table.capacity(),
            index: AtomicUsize::new(0),
            workers: AtomicUsize::new(0),
            finished: AtomicBool::new(false),
            old_table,
        }
    }

    /// Helps with the resize
    ///
    /// will block until the resize is complete.
    pub fn resize(&self)
    {
        // increase worker count by one, since this thread is about to work on the resize
        self.workers.fetch_add(1, Ordering::SeqCst);

        self.do_resize();

        // decrease worker count by one because this thread is done helping.
        let mut workers = self.workers.fetch_sub(1, Ordering::SeqCst) - 1;

        // wait until all workers are finished resizing
        while workers != 0
        {
            workers = self.workers.load(Ordering::SeqCst);
        }
    }

    pub fn finalize(&self, old_table: &Keep<Table<Key, Val>>)
    {
        if !self.finished.swap(true, Ordering::SeqCst)
        {
            old_table.swap_with(&self.new_table);
        }
    }

    fn do_resize(&self)
    {
        loop
        {
            let start_index = self.index.fetch_add(self.stride, Ordering::SeqCst);
            let end_index = (start_index + self.stride).min(self.old_capacity);

            if start_index > end_index
            {
                break;
            }

            let new_table = self.new_table.read();

            for entry in &self.old_table.entries()[start_index..end_index]
            {
                if let Entry::Head(head) = &*entry.read()
                {
                    new_table.insert(head.read().clone_striped());

                    let mut current = head.read().next().read();

                    while let Some(next) = &*current
                    {
                        let next = next.read();
                        new_table.insert(next.clone_striped());
                        current = next.next().read();
                    }
                }
            }
        }
    }
}

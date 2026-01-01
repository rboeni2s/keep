use crate::{
    entry::EntryNode,
    table::{Table, TableIter},
};
use keep::*;
use std::hash::{BuildHasher, Hash, RandomState};


pub struct PlugMap<Key, Val, S = RandomState>
{
    table: Keep<Table<Key, Val>>,
    hasher: Guard<S>,
}


impl<Key, Val, S> PlugMap<Key, Val, S>
{
    pub const DEFAULT_SIZE: usize = 4;
}


impl<Key, Val, S> PlugMap<Key, Val, S>
where
    Key: Hash + Eq,
    S: BuildHasher,
{
    /// Creates a new PlugMap with a capacity of `2^size` and a `BuildHasher` provided by the caller.
    pub fn new_with_hasher(size: usize, hasher: S) -> Self
    {
        Self {
            table: Keep::new(Table::new(size)),
            hasher: Keep::new(hasher).read(),
        }
    }

    /// Tries to remove an entry from the map.
    pub fn remove(&self, key: &Key) -> Option<Keep<Val>>
    {
        self.table.read().remove(key, self.hash(key))
    }

    /// Inserts a new key-value pair into the map or updates an existing one...
    pub fn insert(&self, key: Key, val: impl Heaped<Val>) -> Option<Keep<Val>>
    {
        let hash = self.hash(&key);
        let entry_node = EntryNode::new(key, val, hash);
        self.table.read().insert(entry_node).0
    }

    /// Tries to get a value associated with `key`. Returns `None` if no such value exists.
    pub fn get(&self, key: &Key) -> Option<Guard<Val>>
    {
        self.table.read().get(key, self.hash(key))
    }

    #[inline]
    fn hash(&self, val: impl Hash) -> u64
    {
        self.hasher.hash_one(val)
    }
}


impl<Key, Val> PlugMap<Key, Val, RandomState>
where
    Key: Hash + Eq,
{
    pub fn new() -> Self
    {
        Self::new_with_hasher(Self::DEFAULT_SIZE, RandomState::new())
    }
}


impl<Key, Val> Clone for PlugMap<Key, Val, RandomState>
where
    Key: Hash + Eq,
{
    fn clone(&self) -> Self
    {
        Self {
            table: self.table.clone(),
            hasher: self.hasher.clone(),
        }
    }
}


impl<Key, Val> Default for PlugMap<Key, Val, RandomState>
where
    Key: Hash + Eq,
{
    fn default() -> Self
    {
        Self::new()
    }
}


impl<Key, Val, S> IntoIterator for &PlugMap<Key, Val, S>
where
    Key: Eq,
{
    type Item = Guard<Val>;
    type IntoIter = TableIter<Key, Val>;

    fn into_iter(self) -> Self::IntoIter
    {
        TableIter {
            table: self.table.read(),
            index: 0,
            bin_buffer: vec![],
        }
    }
}


impl<Key, Val, S> IntoIterator for PlugMap<Key, Val, S>
where
    Key: Eq,
{
    type Item = Guard<Val>;
    type IntoIter = TableIter<Key, Val>;

    fn into_iter(self) -> Self::IntoIter
    {
        TableIter {
            table: self.table.read(),
            index: 0,
            bin_buffer: vec![],
        }
    }
}

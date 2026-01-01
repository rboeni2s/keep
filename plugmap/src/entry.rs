use keep::*;


pub enum Entry<Key, Val>
{
    Empty,
    Head(Keep<EntryNode<Key, Val>>),
}


impl<Key, Val> Entry<Key, Val>
where
    Key: Eq,
{
    pub fn search(&self, key: &Key) -> Option<Guard<Val>>
    {
        match self
        {
            Entry::Empty => None,
            Entry::Head(keep) => keep.read().search(key),
        }
    }

    pub fn remove_from_children(&self, key: &Key) -> Option<Keep<Val>>
    {
        match self
        {
            Entry::Empty => None,

            Entry::Head(keep) =>
            {
                todo!("Implement remove");
                // let current = keep;

                // loop
                // {}
            }
        }
    }

    pub fn buffered(&self) -> Vec<Guard<Val>>
    {
        let mut ret = vec![];

        if let Self::Head(head) = self
        {
            head.read().buffered(&mut ret);
        }

        ret
    }
}


pub struct EntryNode<Key, Val>
{
    val: Keep<Val>,
    key: Guard<Key>,
    hash: u64,
    next: Keep<Option<Keep<EntryNode<Key, Val>>>>,
}


impl<Key, Val> EntryNode<Key, Val>
where
    Key: Eq,
{
    #[inline]
    pub fn value(&self) -> &Keep<Val>
    {
        &self.val
    }

    #[inline]
    pub fn next(&self) -> &Keep<Option<Keep<EntryNode<Key, Val>>>>
    {
        &self.next
    }

    #[inline]
    pub fn hash(&self) -> u64
    {
        self.hash
    }

    #[inline]
    pub fn key(&self) -> &Key
    {
        &self.key
    }

    pub fn clone_striped(&self) -> Self
    {
        Self {
            val: self.val.clone(),
            key: self.key.clone(),
            hash: self.hash,
            next: Keep::new(None),
        }
    }

    pub fn new(key: Key, val: impl Heaped<Val>, hash: u64) -> Self
    {
        Self {
            val: Keep::new(val),
            key: Keep::new(key).read(),
            hash,
            next: Keep::new(None),
        }
    }

    pub fn update(&self, node: &Keep<EntryNode<Key, Val>>) -> Option<Keep<Val>>
    {
        if self.key.as_ref() == node.read().key.as_ref()
        {
            let old = self.val.clone_from(&node.read().val);
            return Some(old);
        }

        let next = &self.next;
        let mut next_guard = next.read();

        loop
        {
            match &*next_guard
            {
                Some(next) => return next.read().update(node),

                None =>
                {
                    match next.exchange(&next_guard, Some(node.clone()))
                    {
                        Ok(_old) => return None,

                        Err(actual) =>
                        {
                            next_guard = actual;
                        }
                    }
                }
            }
        }
    }

    pub fn search(&self, key: &Key) -> Option<Guard<Val>>
    {
        if &*self.key == key
        {
            return Some(self.value().read());
        }

        match &*self.next.read()
        {
            Some(next) => next.read().search(key),
            None => None,
        }
    }

    pub fn buffered(&self, buffer: &mut Vec<Guard<Val>>)
    {
        buffer.push(self.value().read());

        if let Some(next) = &*self.next.read()
        {
            next.read().buffered(buffer);
        }
    }
}

use std::sync::atomic::{AtomicUsize, Ordering};


pub struct Drops<'a>(&'a DropChecker);
impl<'a> Drops<'a>
{
    pub fn can_access(&self) -> ()
    {
        ()
    }
}

impl<'a> Drop for Drops<'a>
{
    fn drop(&mut self)
    {
        self.0.0.fetch_sub(1, Ordering::Release);
    }
}


#[derive(Default)]
pub struct DropChecker(AtomicUsize);
impl DropChecker
{
    pub fn new() -> Self
    {
        Self::default()
    }

    pub fn drops(&self) -> Drops<'_>
    {
        self.0.fetch_add(1, Ordering::Release);
        Drops(&self)
    }

    pub fn check(&self) -> usize
    {
        self.0.load(Ordering::Acquire)
    }
}

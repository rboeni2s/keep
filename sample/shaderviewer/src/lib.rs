#[macro_use]
extern crate log;

pub mod service;
pub mod window;


#[macro_export]
macro_rules! rel {
    ($path:expr) => {
        concat!(env!("CARGO_MANIFEST_DIR"), $path)
    };
}


pub enum MaybeOwned<'a, T>
{
    Owned(T),
    Borrowed(&'a T),
}


impl<'a, T> AsRef<T> for MaybeOwned<'a, T>
{
    fn as_ref(&self) -> &T
    {
        match self
        {
            MaybeOwned::Owned(val) => val,
            MaybeOwned::Borrowed(val) => val,
        }
    }
}


impl<'a, T> std::ops::Deref for MaybeOwned<'a, T>
{
    type Target = T;

    fn deref(&self) -> &T
    {
        self.as_ref()
    }
}


impl<T> From<T> for MaybeOwned<'static, T>
{
    fn from(value: T) -> Self
    {
        Self::Owned(value)
    }
}


impl<'a, T> From<&'a T> for MaybeOwned<'a, T>
{
    fn from(value: &'a T) -> Self
    {
        Self::Borrowed(value)
    }
}

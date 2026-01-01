/// Holds a pointer to a value on the heap.
///
/// A `Heap<T>` does not free `T` on drop.
pub struct Heap<T>(*mut T);


impl<T> Heap<T>
{
    /// Creates a new `Heap<T>` from a pointer to a `T` on the heap.
    ///
    /// # Safety
    /// The caller needs to ensure that `ptr` is indeed valid a pointer to a
    /// `T` on the heap. This `T` must not be freed/dropped by anything other than `Heap::free`.
    #[inline]
    pub unsafe fn from_ptr(ptr: *mut T) -> Self
    {
        Self(ptr)
    }

    /// Drops and frees the contained `T`
    ///
    /// # Safety
    /// The caller needs to ensure that one value is never freed more than once
    /// and is also never again used after being freed.
    #[inline]
    pub unsafe fn free(self)
    {
        drop(unsafe { Box::from_raw(self.0) })
    }

    #[inline]
    pub fn as_ptr(&self) -> *mut T
    {
        self.0
    }
}


impl<T> Copy for Heap<T> {}
impl<T> Clone for Heap<T>
{
    fn clone(&self) -> Self
    {
        *self
    }
}


impl<T> AsRef<T> for Heap<T>
{
    fn as_ref(&self) -> &T
    {
        self
    }
}


impl<T> std::ops::Deref for Heap<T>
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        assert!(self.0.is_aligned(), "Pointer was not aligned");
        unsafe { &*self.0 }
    }
}


unsafe impl<T> Send for Heap<T> where T: Sync {}
unsafe impl<T> Sync for Heap<T> where T: Sync {}


/// Provides a method to move the implementing object of `T` on the heap and returns a `Heap<T>` to it.
pub trait Heaped<T>
{
    /// Moves `self` into the heap and returns a `Heap<T>` pointing to a `T`.
    ///
    /// # Safety
    /// Calling `Heaped::heaped` on a value will move it onto the heap without ever
    /// dropping it again, leaking `T` if `Heap::free` is never manually called.
    /// Therefore the caller needs to ensure that `T` is eventually correctly dropped.
    unsafe fn heaped(self) -> Heap<T>;
}


impl<T> Heaped<T> for T
{
    unsafe fn heaped(self) -> Heap<T>
    {
        Heap(Box::into_raw(Box::new(self)))
    }
}


impl<T> Heaped<T> for Box<T>
{
    unsafe fn heaped(self) -> Heap<T>
    {
        Heap(Box::into_raw(self))
    }
}


impl<T> Heaped<T> for Heap<T>
{
    unsafe fn heaped(self) -> Heap<T>
    {
        self
    }
}

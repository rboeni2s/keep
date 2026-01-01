use std::sync::atomic::{AtomicPtr, Ordering};

use crate::{
    guard::Guard,
    heaped::{Heap, Heaped},
    tracked_atomic::TrackedAtomic,
};


pub struct KeepMarker<T>(*mut TrackedAtomic<T>);


pub struct Keep<T>
{
    tracked_atomic: AtomicPtr<AtomicPtr<TrackedAtomic<T>>>,
}


impl<T> Keep<T>
{
    pub fn new(val: impl Heaped<T>) -> Self
    {
        let me = Self {
            tracked_atomic: AtomicPtr::new(unsafe {
                AtomicPtr::new(TrackedAtomic::new(val).as_ptr())
                    .heaped()
                    .as_ptr()
            }),
        };

        unsafe {
            &*me.tracked_atomic
                .load(Ordering::Acquire)
                .as_ref()
                .unwrap()
                .load(Ordering::Acquire)
        }
        .register_accessor();
        me
    }

    pub fn read(&self) -> Guard<T>
    {
        unsafe {
            &*self
                .tracked_atomic
                .load(Ordering::Acquire)
                .as_ref()
                .unwrap()
                .load(Ordering::Acquire)
        }
        .load()
    }

    pub fn read_marked(&self) -> (Guard<T>, KeepMarker<T>)
    {
        let tracked_atomic = unsafe {
            self.tracked_atomic
                .load(Ordering::Acquire)
                .as_ref()
                .unwrap()
                .load(Ordering::Acquire)
        };

        let marker = KeepMarker(tracked_atomic);
        let guard = unsafe { &*tracked_atomic }.load();

        (guard, marker)
    }

    pub fn write(&self, val: impl Heaped<T>)
    {
        unsafe {
            &*self
                .tracked_atomic
                .load(Ordering::Acquire)
                .as_ref()
                .unwrap()
                .load(Ordering::Acquire)
        }
        .store(val)
    }

    pub fn swap(&self, new_value: impl Heaped<T>) -> Guard<T>
    {
        unsafe {
            &*self
                .tracked_atomic
                .load(Ordering::Acquire)
                .as_ref()
                .unwrap()
                .load(Ordering::Acquire)
        }
        .swap(new_value)
    }

    pub fn exchange(
        &self,
        current: &Guard<T>,
        new_value: impl Heaped<T>,
    ) -> Result<Guard<T>, Guard<T>>
    {
        unsafe {
            &*self
                .tracked_atomic
                .load(Ordering::Acquire)
                .as_ref()
                .unwrap()
                .load(Ordering::Acquire)
        }
        .exchange(current, new_value)
    }

    pub fn swap_with(&self, other: &Keep<T>)
    {
        let a = unsafe {
            self.tracked_atomic
                .load(Ordering::Acquire)
                .as_ref()
                .unwrap()
                .load(Ordering::Acquire)
        };
        let b = unsafe {
            other
                .tracked_atomic
                .load(Ordering::Acquire)
                .as_ref()
                .unwrap()
                .swap(a, Ordering::SeqCst)
        };
        unsafe {
            self.tracked_atomic
                .load(Ordering::Acquire)
                .as_ref()
                .unwrap()
                .store(b, Ordering::Release)
        };
    }

    pub fn swap_with_marked(&self, marker: KeepMarker<T>, other: &Keep<T>) -> bool
    {
        let other_ta = unsafe {
            other
                .tracked_atomic
                .load(Ordering::Acquire)
                .as_ref()
                .unwrap()
                .load(Ordering::Acquire)
        };

        if let Ok(self_ta) = unsafe {
            self.tracked_atomic
                .load(Ordering::Acquire)
                .as_ref()
                .unwrap()
                .compare_exchange(marker.0, other_ta, Ordering::AcqRel, Ordering::Relaxed)
        }
        {
            unsafe {
                other
                    .tracked_atomic
                    .load(Ordering::Acquire)
                    .as_ref()
                    .unwrap()
                    .store(self_ta, Ordering::Release)
            };

            return true;
        }

        false
    }

    /// Clones `other` into `self` and returns the "old" `self`
    pub fn clone_from(&self, other: &Keep<T>) -> Self
    {
        // Read the tracked atomic from other
        let other_ta = unsafe {
            other
                .tracked_atomic
                .load(Ordering::Acquire)
                .as_ref()
                .unwrap()
                .load(Ordering::Acquire)
        };

        // Increase the accessor count of other
        unsafe { &*other_ta }.register_accessor();

        // Replace the "old" tracked atomic in self with other
        let old = self.tracked_atomic.swap(
            other.tracked_atomic.load(Ordering::Acquire),
            Ordering::SeqCst,
        );

        Keep {
            tracked_atomic: AtomicPtr::new(old),
        }
    }

    unsafe fn destroy(&self)
    {
        let tracked_atomic = unsafe {
            Heap::from_ptr(
                self.tracked_atomic
                    .load(Ordering::Acquire)
                    .as_ref()
                    .unwrap()
                    .load(Ordering::Acquire),
            )
        };

        if tracked_atomic.unregister_accessor() && tracked_atomic.is_dead()
        {
            unsafe {
                tracked_atomic.destroy();
                tracked_atomic.free();
                Heap::from_ptr(self.tracked_atomic.load(Ordering::Acquire)).free();
            };
        }
    }
}


impl<T> Clone for Keep<T>
{
    fn clone(&self) -> Self
    {
        unsafe {
            Heap::from_ptr(
                self.tracked_atomic
                    .load(Ordering::Acquire)
                    .as_ref()
                    .unwrap()
                    .load(Ordering::Acquire),
            )
        }
        .register_accessor();

        Self {
            tracked_atomic: AtomicPtr::new(self.tracked_atomic.load(Ordering::Acquire)),
        }
    }
}


impl<T> Drop for Keep<T>
{
    fn drop(&mut self)
    {
        unsafe { self.destroy() };
    }
}

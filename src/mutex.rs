use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
};

pub struct Mutex<T: ?Sized> {
    locked: UnsafeCell<bool>,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}

impl<T> Mutex<T> {
    #[inline(always)]
    pub const fn new(data: T) -> Mutex<T> {
        Mutex {
            locked: UnsafeCell::new(false),
            data: UnsafeCell::new(data),
        }
    }

    #[inline(always)]
    pub fn lock(&self) -> MutexGuard<T> {
        unsafe {
            let locked_ptr = self.locked.get();
            if *locked_ptr {
                panic!("Mutex is already locked");
            }
            *locked_ptr = true;
        }
        MutexGuard {
            locked: self.locked.get(),
            data: self.data.get(),
        }
    }
}

pub struct MutexGuard<T: ?Sized> {
    locked: *mut bool,
    data: *mut T,
}

impl<T: ?Sized> Deref for MutexGuard<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.data }
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.data }
    }
}

impl<T: ?Sized> Drop for MutexGuard<T> {
    fn drop(&mut self) {
        unsafe { *self.locked = false }
    }
}

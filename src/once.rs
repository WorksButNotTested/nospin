use core::{cell::UnsafeCell, convert::Infallible, mem::MaybeUninit};

pub struct Once<T = ()> {
    initialized: UnsafeCell<bool>,
    data: UnsafeCell<MaybeUninit<T>>,
}

unsafe impl<T: Send + Sync> Sync for Once<T> {}
unsafe impl<T: Send> Send for Once<T> {}

impl<T> Once<T> {
    #[allow(clippy::declare_interior_mutable_const)]
    pub const INIT: Self = Self {
        initialized: UnsafeCell::new(false),
        data: UnsafeCell::new(MaybeUninit::uninit()),
    };

    pub const fn new() -> Self {
        Self::INIT
    }

    fn is_initialized(&self) -> bool {
        unsafe { *self.initialized.get() }
    }

    unsafe fn force_get(&self) -> &T {
        unsafe { &*(*self.data.get()).as_ptr() }
    }

    unsafe fn force_get_mut(&mut self) -> &mut T {
        unsafe { &mut *(*self.data.get()).as_mut_ptr() }
    }

    pub fn call_once<F: FnOnce() -> T>(&self, f: F) -> &T {
        self.try_call_once(|| Ok::<T, Infallible>(f())).unwrap()
    }

    pub fn try_call_once<F: FnOnce() -> Result<T, E>, E>(&self, f: F) -> Result<&T, E> {
        unsafe {
            if self.is_initialized() {
                Ok(&*(*self.data.get()).as_ptr())
            } else {
                let value = f()?;
                (*self.data.get()).as_mut_ptr().write(value);
                *self.initialized.get() = true;
                Ok(&*(*self.data.get()).as_ptr())
            }
        }
    }

    pub fn get(&self) -> Option<&T> {
        unsafe { self.is_initialized().then(|| self.force_get()) }
    }

    pub fn get_mut(&mut self) -> Option<&mut T> {
        unsafe { self.is_initialized().then(|| self.force_get_mut()) }
    }
}

impl<T> Default for Once<T> {
    fn default() -> Self {
        Self::new()
    }
}

use {
    alloc::fmt,
    core::{cell::UnsafeCell, convert::Infallible, mem::MaybeUninit},
};

/// A primitive that provides lazy one-time initialization.
///
/// Unlike its `std::sync` equivalent, this is generalized such that the closure returns a
/// value to be stored by the [`Once`] (`std::sync::Once` can be trivially emulated with
/// `Once`).
///
/// Because [`Once::new`] is `const`, this primitive may be used to safely initialize statics.
///
/// # Examples
///
/// ```
/// use nospin;
///
/// static START: nospin::Once = nospin::Once::new();
///
/// START.call_once(|| {
///     // run initialization here
/// });
/// ```
pub struct Once<T = ()> {
    initialized: UnsafeCell<bool>,
    panicked: UnsafeCell<bool>,
    data: UnsafeCell<MaybeUninit<T>>,
}

impl<T: fmt::Debug> fmt::Debug for Once<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut d = f.debug_tuple("Once");
        let d = if let Some(x) = self.get() {
            d.field(&x)
        } else {
            d.field(&format_args!("<uninit>"))
        };
        d.finish()
    }
}

impl<T> From<T> for Once<T> {
    fn from(data: T) -> Self {
        Self::initialized(data)
    }
}

impl<T> Drop for Once<T> {
    fn drop(&mut self) {
        // No need to do any atomic access here, we have &mut!
        if self.is_completed() {
            unsafe {
                //TODO: Use MaybeUninit::assume_init_drop once stabilised
                core::ptr::drop_in_place((*self.data.get()).as_mut_ptr());
            }
        }
    }
}

unsafe impl<T: Send + Sync> Sync for Once<T> {}
unsafe impl<T: Send> Send for Once<T> {}

impl<T> Once<T> {
    #[allow(clippy::declare_interior_mutable_const)]
    pub const INIT: Self = Self {
        initialized: UnsafeCell::new(false),
        panicked: UnsafeCell::new(false),
        data: UnsafeCell::new(MaybeUninit::uninit()),
    };

    pub const fn new() -> Self {
        Self::INIT
    }

    /// Retrieve a pointer to the inner data.
    ///
    /// While this method itself is safe, accessing the pointer before the [`Once`] has been
    /// initialized is UB, unless this method has already been written to from a pointer coming
    /// from this method.
    pub fn as_mut_ptr(&self) -> *mut T {
        // SAFETY:
        // * MaybeUninit<T> always has exactly the same layout as T
        self.data.get().cast::<T>()
    }

    /// Get a reference to the initialized instance. Must only be called once COMPLETE.
    unsafe fn force_get(&self) -> &T {
        // SAFETY:
        // * `UnsafeCell`/inner deref: data never changes again
        // * `MaybeUninit`/outer deref: data was initialized
        unsafe { &*(*self.data.get()).as_ptr() }
    }

    /// Get a reference to the initialized instance. Must only be called once COMPLETE.
    unsafe fn force_get_mut(&mut self) -> &mut T {
        // SAFETY:
        // * `UnsafeCell`/inner deref: data never changes again
        // * `MaybeUninit`/outer deref: data was initialized
        unsafe { &mut *(*self.data.get()).as_mut_ptr() }
    }

    /// Get a reference to the initialized instance. Must only be called once COMPLETE.
    unsafe fn force_into_inner(self) -> T {
        // SAFETY:
        // * `UnsafeCell`/inner deref: data never changes again
        // * `MaybeUninit`/outer deref: data was initialized
        unsafe { (*self.data.get()).as_ptr().read() }
    }

    /// Performs an initialization routine once and only once. The given closure
    /// will be executed if this is the first time `call_once` has been called,
    /// and otherwise the routine will *not* be invoked.
    ///
    /// The behaviour of this function is undefined in multi-threaded environments.
    ///
    /// When this function returns, it is guaranteed that some initialization
    /// has run and completed (it may not be the closure specified). The
    /// returned pointer will point to the result from the closure that was
    /// run.
    ///
    /// # Panics
    ///
    /// This function will panic if the [`Once`] previously panicked while attempting
    /// to initialize. This is similar to the poisoning behaviour of `std::sync`'s
    /// primitives.
    ///
    /// # Examples
    ///
    /// ```
    /// use nospin;
    ///
    /// static INIT: nospin::Once<usize> = nospin::Once::new();
    ///
    /// fn get_cached_val() -> usize {
    ///     *INIT.call_once(expensive_computation)
    /// }
    ///
    /// fn expensive_computation() -> usize {
    ///     // ...
    /// # 2
    /// }
    /// ```
    pub fn call_once<F: FnOnce() -> T>(&self, f: F) -> &T {
        match self.try_call_once(|| Ok::<T, Infallible>(f())) {
            Ok(x) => x,
            Err(void) => match void {},
        }
    }

    /// This method is similar to `call_once`, but allows the given closure to
    /// fail, and lets the `Once` in a uninitialized state if it does.
    ///
    /// This method is NOT thread safe
    ///
    /// When this function returns without error, it is guaranteed that some
    /// initialization has run and completed (it may not be the closure
    /// specified). The returned reference will point to the result from the
    /// closure that was run.
    ///
    /// # Panics
    ///
    /// This function will panic if the [`Once`] previously panicked while attempting
    /// to initialize. This is similar to the poisoning behaviour of `std::sync`'s
    /// primitives.
    ///
    /// # Examples
    ///
    /// ```
    /// use nospin;
    ///
    /// static INIT: nospin::Once<usize> = nospin::Once::new();
    ///
    /// fn get_cached_val() -> Result<usize, String> {
    ///     INIT.try_call_once(expensive_fallible_computation).map(|x| *x)
    /// }
    ///
    /// fn expensive_fallible_computation() -> Result<usize, String> {
    ///     // ...
    /// # Ok(2)
    /// }
    /// ```
    pub fn try_call_once<F: FnOnce() -> Result<T, E>, E>(&self, f: F) -> Result<&T, E> {
        unsafe {
            if *self.panicked.get() {
                panic!("Initialization panicked");
            } else if self.is_completed() {
                Ok(self.force_get())
            } else {
                *self.panicked.get() = true;
                let value = f();
                *self.panicked.get() = false;
                (*self.data.get()).as_mut_ptr().write(value?);
                *self.initialized.get() = true;
                Ok(self.force_get())
            }
        }
    }

    /// Returns a reference to the inner value if the [`Once`] has been initialized.
    pub fn get(&self) -> Option<&T> {
        unsafe { self.is_completed().then(|| self.force_get()) }
    }

    /// Returns a mutable reference to the inner value if the [`Once`] has been initialized.
    ///
    /// Because this method requires a mutable reference to the [`Once`], no synchronization
    /// overhead is required to access the inner value. In effect, it is zero-cost.
    pub fn get_mut(&mut self) -> Option<&mut T> {
        unsafe { self.is_completed().then(|| self.force_get_mut()) }
    }

    /// Returns a mutable reference to the inner value
    ///
    /// # Safety
    ///
    /// This is *extremely* unsafe if the `Once` has not already been initialized because a reference to uninitialized
    /// memory will be returned, immediately triggering undefined behaviour (even if the reference goes unused).
    /// However, this can be useful in some instances for exposing the `Once` to FFI or when the overhead of atomically
    /// checking initialization is unacceptable and the `Once` has already been initialized.
    pub unsafe fn get_mut_unchecked(&mut self) -> &mut T {
        debug_assert!(
            self.is_completed(),
            "Attempted to access an unintialized Once.  If this was to run without debug checks, this would be undefined behavior.  This is a serious bug and you must fix it.",
        );
        unsafe { self.force_get_mut() }
    }

    /// Returns a reference to the inner value on the unchecked assumption that the  [`Once`] has been initialized.
    ///
    /// # Safety
    ///
    /// This is *extremely* unsafe if the `Once` has not already been initialized because a reference to uninitialized
    /// memory will be returned, immediately triggering undefined behaviour (even if the reference goes unused).
    /// However, this can be useful in some instances for exposing the `Once` to FFI or when the overhead of atomically
    /// checking initialization is unacceptable and the `Once` has already been initialized.
    pub unsafe fn get_unchecked(&self) -> &T {
        debug_assert!(
            self.is_completed(),
            "Attempted to access an unintialized Once.  If this was to run without debug checks, this would be undefined behavior.  This is a serious bug and you must fix it.",
        );
        unsafe { self.force_get() }
    }

    /// Creates a new initialized [`Once`].
    pub const fn initialized(data: T) -> Self {
        Self {
            initialized: UnsafeCell::new(true),
            panicked: UnsafeCell::new(false),
            data: UnsafeCell::new(MaybeUninit::new(data)),
        }
    }

    /// Returns a the inner value if the [`Once`] has been initialized.
    /// # Safety
    ///
    /// This is *extremely* unsafe if the `Once` has not already been initialized because a reference to uninitialized
    /// memory will be returned, immediately triggering undefined behaviour (even if the reference goes unused)
    /// This can be useful, if `Once` has already been initialized, and you want to bypass an
    /// option check.
    pub unsafe fn into_inner_unchecked(self) -> T {
        debug_assert!(
            self.is_completed(),
            "Attempted to access an unintialized Once.  If this was to run without debug checks, this would be undefined behavior.  This is a serious bug and you must fix it.",
        );
        unsafe { self.force_into_inner() }
    }

    /// Checks whether the value has been initialized.
    ///
    /// It is safe to access the value directly via [`get_unchecked`](Self::get_unchecked) if this returns true.
    pub fn is_completed(&self) -> bool {
        unsafe { *self.initialized.get() }
    }

    /// Behaves as [`Once::get`], but provided for API compatibility with `spin``.
    ///
    /// # Panics
    ///
    /// This function will panic if the [`Once`] previously panicked while attempting
    /// to initialize. This is similar to the poisoning behaviour of `std::sync`'s
    /// primitives.
    pub fn poll(&self) -> Option<&T> {
        self.get()
    }

    /// This function makes no sense in a single-threaded environment. It is provided for
    /// API compatibility with `spin`, but will simply panic if the [`Once`] hasn't
    /// been initialized.
    ///
    /// # Panics
    ///
    /// This function will panic if the [`Once`] previously panicked while attempting
    /// to initialize. This is similar to the poisoning behaviour of `std::sync`'s
    /// primitives.
    pub fn wait(&self) -> &T {
        self.get()
            .expect("Waited on uninitialized Once, who are you waiting for?")
    }
}

impl<T> Default for Once<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::prelude::v1::*;
    use std::sync::atomic::Ordering;

    use std::sync::atomic::AtomicU32;
    use std::sync::mpsc::channel;
    use std::thread;

    use super::*;

    #[test]
    fn smoke_once() {
        static O: Once = Once::new();
        let mut a = 0;
        O.call_once(|| a += 1);
        assert_eq!(a, 1);
        O.call_once(|| a += 1);
        assert_eq!(a, 1);
    }

    #[test]
    fn smoke_once_value() {
        static O: Once<usize> = Once::new();
        let a = O.call_once(|| 1);
        assert_eq!(*a, 1);
        let b = O.call_once(|| 2);
        assert_eq!(*b, 1);
    }

    #[test]
    fn stampede_once() {
        static O: Once = Once::new();
        static mut RUN: bool = false;

        let (tx, rx) = channel();
        let mut ts = Vec::new();
        for _ in 0..10 {
            let tx = tx.clone();
            ts.push(thread::spawn(move || {
                for _ in 0..4 {
                    thread::yield_now()
                }
                unsafe {
                    O.call_once(|| {
                        assert!(!RUN);
                        RUN = true;
                    });
                    assert!(RUN);
                }
                tx.send(()).unwrap();
            }));
        }

        unsafe {
            O.call_once(|| {
                assert!(!RUN);
                RUN = true;
            });
            assert!(RUN);
        }

        for _ in 0..10 {
            rx.recv().unwrap();
        }

        for t in ts {
            t.join().unwrap();
        }
    }

    #[test]
    fn get() {
        static INIT: Once<usize> = Once::new();

        assert!(INIT.get().is_none());
        INIT.call_once(|| 2);
        assert_eq!(INIT.get().copied(), Some(2));
    }

    #[test]
    fn get_no_wait() {
        static INIT: Once<usize> = Once::new();

        assert!(INIT.get().is_none());
        let t = thread::spawn(move || {
            INIT.call_once(|| {
                thread::sleep(std::time::Duration::from_secs(3));
                42
            });
        });
        assert!(INIT.get().is_none());

        t.join().unwrap();
    }

    #[test]
    fn poll() {
        static INIT: Once<usize> = Once::new();

        assert!(INIT.poll().is_none());
        INIT.call_once(|| 3);
        assert_eq!(INIT.poll().copied(), Some(3));
    }

    #[test]
    fn wait() {
        static INIT: Once<usize> = Once::new();

        let t = std::thread::spawn(|| {
            assert_eq!(*INIT.wait(), 3);
            assert!(INIT.is_completed());
        });

        for _ in 0..4 {
            thread::yield_now()
        }

        assert!(INIT.poll().is_none());
        INIT.call_once(|| 3);

        t.join().unwrap();
    }

    #[test]
    fn panic() {
        use std::panic;

        static INIT: Once = Once::new();

        // poison the once
        let t = panic::catch_unwind(|| {
            INIT.call_once(|| panic!());
        });
        assert!(t.is_err());

        // poisoning propagates
        let t = panic::catch_unwind(|| {
            INIT.call_once(|| {});
        });
        assert!(t.is_err());
    }

    #[test]
    fn init_constant() {
        static O: Once = Once::INIT;
        let mut a = 0;
        O.call_once(|| a += 1);
        assert_eq!(a, 1);
        O.call_once(|| a += 1);
        assert_eq!(a, 1);
    }

    static mut CALLED: bool = false;

    struct DropTest {}

    impl Drop for DropTest {
        fn drop(&mut self) {
            unsafe {
                CALLED = true;
            }
        }
    }

    #[test]
    fn try_call_once_err() {
        let once = Once::<_>::new();
        let called = AtomicU32::new(0);

        once.try_call_once(|| {
            called.fetch_add(1, Ordering::AcqRel);
            thread::sleep(std::time::Duration::from_millis(50));
            Err(())
        })
        .ok();

        once.call_once(|| {
            called.fetch_add(1, Ordering::AcqRel);
        });

        assert_eq!(called.load(Ordering::Acquire), 2);
    }

    // This is sort of two test cases, but if we write them as separate test methods
    // they can be executed concurrently and then fail some small fraction of the
    // time.
    #[test]
    fn drop_occurs_and_skip_uninit_drop() {
        unsafe {
            CALLED = false;
        }

        {
            let once = Once::<_>::new();
            once.call_once(|| DropTest {});
        }

        assert!(unsafe { CALLED });
        // Now test that we skip drops for the uninitialized case.
        unsafe {
            CALLED = false;
        }

        let once = Once::<DropTest>::new();
        drop(once);

        assert!(unsafe { !CALLED });
    }

    #[test]
    fn call_once_test() {
        for _ in 0..20 {
            use std::sync::Arc;
            use std::sync::atomic::AtomicUsize;
            let share = Arc::new(AtomicUsize::new(0));
            let once = Arc::new(Once::<_>::new());
            for _ in 0..8 {
                once.call_once(|| {
                    share.fetch_add(1, Ordering::SeqCst);
                });
            }
            assert_eq!(1, share.load(Ordering::SeqCst));
        }
    }
}

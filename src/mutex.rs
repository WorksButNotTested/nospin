use {
    alloc::fmt,
    core::{
        cell::UnsafeCell,
        ops::{Deref, DerefMut},
    },
};

/// A Mutex that is NOT thread safe allow uncontested access to mutable data
/// suitable only for single-threaded environments.
///
/// # Example
///
/// ```
/// use nospin;
///
/// let lock = nospin::Mutex::new(0);
///
/// // Modify the data
/// *lock.lock() = 2;
///
/// // Read the data
/// let answer = *lock.lock();
/// assert_eq!(answer, 2);
/// ```
pub struct Mutex<T: ?Sized> {
    locked: UnsafeCell<bool>,
    data: UnsafeCell<T>,
}

impl<T: fmt::Debug> fmt::Debug for Mutex<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_lock() {
            Some(guard) => write!(f, "Mutex {{ data: ")
                .and_then(|()| (*guard).fmt(f))
                .and_then(|()| write!(f, " }}")),
            None => write!(f, "Mutex {{ <locked> }}"),
        }
    }
}

impl<T: Default> Default for Mutex<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T> From<T> for Mutex<T> {
    fn from(data: T) -> Self {
        Self::new(data)
    }
}

unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}

impl<T> Mutex<T> {
    /// Creates a new [`Mutex`] wrapping the supplied data.
    ///
    /// # Example
    ///
    /// ```
    /// use nospin::Mutex;
    ///
    /// static MUTEX: Mutex<()> = Mutex::new(());
    ///
    /// fn demo() {
    ///     let lock = MUTEX.lock();
    ///     // do something with lock
    ///     drop(lock);
    /// }
    /// ```
    #[inline(always)]
    pub const fn new(data: T) -> Mutex<T> {
        Mutex {
            locked: UnsafeCell::new(false),
            data: UnsafeCell::new(data),
        }
    }

    /// Consumes this [`Mutex`] and unwraps the underlying data.
    ///
    /// # Example
    ///
    /// ```
    /// let lock = nospin::Mutex::new(42);
    /// assert_eq!(42, lock.into_inner());
    /// ```
    #[inline(always)]
    pub fn into_inner(self) -> T {
        // We know statically that there are no outstanding references to
        // `self` so there's no need to lock.
        let Mutex { data, .. } = self;
        data.into_inner()
    }
}

impl<T: ?Sized> Mutex<T> {
    /// Force unlock this [`Mutex`].
    ///
    /// # Safety
    ///
    /// This is *extremely* unsafe if the lock is not held by the current
    /// thread. However, this can be useful in some instances for exposing the
    /// lock to FFI that doesn't know how to deal with RAII.
    #[inline(always)]
    pub unsafe fn force_unlock(&self) {
        unsafe { *self.locked.get() = false };
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// Since this call borrows the [`Mutex`] mutably, and a mutable reference is guaranteed to be exclusive in Rust,
    /// no actual locking needs to take place -- the mutable borrow statically guarantees no locks exist. As such,
    /// this is a 'zero-cost' operation.
    ///
    /// # Example
    ///
    /// ```
    /// let mut lock = nospin::Mutex::new(0);
    /// *lock.get_mut() = 10;
    /// assert_eq!(*lock.lock(), 10);
    /// ```
    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut T {
        // We know statically that there are no other references to `self`, so
        // there's no need to lock the inner mutex.
        unsafe { &mut *self.data.get() }
    }

    /// Returns `true` if the lock is currently held.
    ///
    /// # Safety
    ///
    /// This function provides no synchronization guarantees and so its result should be considered 'out of date'
    /// the instant it is called. Do not use it for synchronization purposes. However, it may be useful as a heuristic.
    #[inline(always)]
    pub fn is_locked(&self) -> bool {
        unsafe { *self.locked.get() }
    }

    /// Locks the [`Mutex`] and returns a guard that permits access to the inner data.
    ///
    /// The returned value may be dereferenced for data access
    /// and the lock will be dropped when the guard falls out of scope.
    ///
    /// ```
    /// let lock = nospin::Mutex::new(0);
    /// {
    ///     let mut data = lock.lock();
    ///     // The lock is now locked and the data can be accessed
    ///     *data += 1;
    ///     // The lock is implicitly dropped at the end of the scope
    /// }
    /// ```
    #[inline(always)]
    pub fn lock(&self) -> MutexGuard<T> {
        if self.is_locked() {
            panic!("Mutex is already locked");
        }
        unsafe {
            *self.locked.get() = true;
        }
        MutexGuard {
            locked: self.locked.get(),
            data: self.data.get(),
        }
    }

    /// Try to lock this [`Mutex`], returning a lock guard if successful.
    ///
    /// # Example
    ///
    /// ```
    /// let lock = nospin::Mutex::new(42);
    ///
    /// let maybe_guard = lock.try_lock();
    /// assert!(maybe_guard.is_some());
    ///
    /// // `maybe_guard` is still held, so the second call fails
    /// let maybe_guard2 = lock.try_lock();
    /// assert!(maybe_guard2.is_none());
    /// ```
    #[inline(always)]
    pub fn try_lock(&self) -> Option<MutexGuard<T>> {
        if self.is_locked() {
            None
        } else {
            unsafe {
                *self.locked.get() = true;
            }
            Some(MutexGuard {
                locked: self.locked.get(),
                data: self.data.get(),
            })
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

#[cfg(feature = "lock_api")]
unsafe impl lock_api_crate::RawMutex for Mutex<()> {
    type GuardMarker = lock_api_crate::GuardSend;

    #[allow(clippy::declare_interior_mutable_const)]
    const INIT: Self = Self::new(());

    fn lock(&self) {
        // Prevent guard destructor running
        core::mem::forget(Self::lock(self));
    }

    fn try_lock(&self) -> bool {
        // Prevent guard destructor running
        Self::try_lock(self).map(core::mem::forget).is_some()
    }

    unsafe fn unlock(&self) {
        unsafe { self.force_unlock() };
    }

    fn is_locked(&self) -> bool {
        self.is_locked()
    }
}

#[cfg(test)]
mod tests {
    use std::prelude::v1::*;

    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc::channel;
    use std::thread;

    type Mutex<T> = super::Mutex<T>;

    #[derive(Eq, PartialEq, Debug)]
    struct NonCopy(i32);

    #[test]
    fn smoke() {
        let m = Mutex::<_>::new(());
        drop(m.lock());
        drop(m.lock());
    }

    #[test]
    fn lots_and_lots() {
        static M: Mutex<()> = Mutex::<_>::new(());
        static mut CNT: u32 = 0;
        const J: u32 = 1000;
        const K: u32 = 3;

        fn inc() {
            for _ in 0..J {
                unsafe {
                    let _g = M.lock();
                    CNT += 1;
                }
            }
        }

        for _ in 0..K {
            inc();
        }

        assert_eq!(unsafe { CNT }, J * K);
    }

    #[test]
    fn try_lock() {
        let mutex = Mutex::<_>::new(42);

        // First lock succeeds
        let a = mutex.try_lock();
        assert_eq!(a.as_ref().map(|r| **r), Some(42));

        // Additional lock fails
        let b = mutex.try_lock();
        assert!(b.is_none());

        // After dropping lock, it succeeds again
        ::core::mem::drop(a);
        let c = mutex.try_lock();
        assert_eq!(c.as_ref().map(|r| **r), Some(42));
    }

    #[test]
    fn test_into_inner() {
        let m = Mutex::<_>::new(NonCopy(10));
        assert_eq!(m.into_inner(), NonCopy(10));
    }

    #[test]
    fn test_into_inner_drop() {
        struct Foo(Arc<AtomicUsize>);
        impl Drop for Foo {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let num_drops = Arc::new(AtomicUsize::new(0));
        let m = Mutex::<_>::new(Foo(num_drops.clone()));
        assert_eq!(num_drops.load(Ordering::SeqCst), 0);
        {
            let _inner = m.into_inner();
            assert_eq!(num_drops.load(Ordering::SeqCst), 0);
        }
        assert_eq!(num_drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_mutex_arc_nested() {
        // Tests nested mutexes and access
        // to underlying data.
        let arc = Arc::new(Mutex::<_>::new(1));
        let arc2 = Arc::new(Mutex::<_>::new(arc));
        let (tx, rx) = channel();
        let t = thread::spawn(move || {
            let lock = arc2.lock();
            let lock2 = lock.lock();
            assert_eq!(*lock2, 1);
            tx.send(()).unwrap();
        });
        rx.recv().unwrap();
        t.join().unwrap();
    }

    #[test]
    fn test_mutex_arc_access_in_unwind() {
        let arc = Arc::new(Mutex::<_>::new(1));
        let arc2 = arc.clone();
        let _ = thread::spawn(move || -> () {
            struct Unwinder {
                i: Arc<Mutex<i32>>,
            }
            impl Drop for Unwinder {
                fn drop(&mut self) {
                    *self.i.lock() += 1;
                }
            }
            let _u = Unwinder { i: arc2 };
            panic!();
        })
        .join();
        let lock = arc.lock();
        assert_eq!(*lock, 2);
    }

    #[test]
    fn test_mutex_unsized() {
        let mutex: &Mutex<[i32]> = &Mutex::<_>::new([1, 2, 3]);
        {
            let b = &mut *mutex.lock();
            b[0] = 4;
            b[2] = 5;
        }
        let comp: &[i32] = &[4, 2, 5];
        assert_eq!(&*mutex.lock(), comp);
    }

    #[test]
    fn test_mutex_force_lock() {
        let lock = Mutex::<_>::new(());
        ::std::mem::forget(lock.lock());
        unsafe {
            lock.force_unlock();
        }
        assert!(lock.try_lock().is_some());
    }
}

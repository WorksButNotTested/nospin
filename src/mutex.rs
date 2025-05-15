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
/// use spin;
///
/// let lock = spin::Mutex::new(0);
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
    #[inline(always)]
    pub const fn new(data: T) -> Mutex<T> {
        Mutex {
            locked: UnsafeCell::new(false),
            data: UnsafeCell::new(data),
        }
    }

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
    /// let mut lock = spin::Mutex::new(0);
    /// *lock.get_mut() = 10;
    /// assert_eq!(*lock.lock(), 10);
    /// ```
    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut T {
        // We know statically that there are no other references to `self`, so
        // there's no need to lock the inner mutex.
        unsafe { &mut *self.data.get() }
    }

    /// Consumes this [`Mutex`] and unwraps the underlying data.
    ///
    /// # Example
    ///
    /// ```
    /// let lock = spin::Mutex::new(42);
    /// assert_eq!(42, lock.into_inner());
    /// ```
    #[inline(always)]
    pub fn into_inner(self) -> T {
        // We know statically that there are no outstanding references to
        // `self` so there's no need to lock.
        let Mutex { data, .. } = self;
        data.into_inner()
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
    /// let lock = spin::Mutex::new(0);
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
    /// let lock = spin::Mutex::new(42);
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

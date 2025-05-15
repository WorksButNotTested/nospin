use {
    crate::once::Once,
    alloc::fmt,
    core::{cell::Cell, ops::Deref},
};

/// A value which is initialized on the first access.
///
/// This type is NOT a thread-safe `Lazy`, and can be used in statics.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use nospin::Lazy;
///
/// static HASHMAP: Lazy<HashMap<i32, String>> = Lazy::new(|| {
///     println!("initializing");
///     let mut m = HashMap::new();
///     m.insert(13, "Spica".to_string());
///     m.insert(74, "Hoyten".to_string());
///     m
/// });
///
/// fn main() {
///     println!("ready");
///     println!("{:?}", HASHMAP.get(&13));
///     println!("{:?}", HASHMAP.get(&74));
///
///     // Prints:
///     //   ready
///     //   initializing
///     //   Some("Spica")
///     //   Some("Hoyten")
/// }
/// ```
pub struct Lazy<T, F = fn() -> T> {
    cell: Once<T>,
    init: Cell<Option<F>>,
}

impl<T: fmt::Debug, F> fmt::Debug for Lazy<T, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_tuple("Lazy");
        let d = if let Some(x) = self.cell.get() {
            d.field(&x)
        } else {
            d.field(&format_args!("<uninit>"))
        };
        d.finish()
    }
}

unsafe impl<T, F: Send> Sync for Lazy<T, F> where Once<T>: Sync {}

impl<T, F> Lazy<T, F> {
    /// Creates a new lazy value with the given initializing
    /// function.
    #[inline(always)]
    pub const fn new(f: F) -> Lazy<T, F> {
        Lazy {
            cell: Once::new(),
            init: Cell::new(Some(f)),
        }
    }

    /// Retrieves a mutable pointer to the inner data.
    ///
    /// This is especially useful when interfacing with low level code or FFI where the caller
    /// explicitly knows that it has exclusive access to the inner data. Note that reading from
    /// this pointer is UB until initialized or directly written to.
    pub fn as_mut_ptr(&self) -> *mut T {
        self.cell.as_mut_ptr()
    }
}

impl<T, F: FnOnce() -> T> Lazy<T, F> {
    /// Forces the evaluation of this lazy value and
    /// returns a reference to result. This is equivalent
    /// to the `Deref` impl, but is explicit.
    ///
    /// # Examples
    ///
    /// ```
    /// use nospin::Lazy;
    ///
    /// let lazy = Lazy::new(|| 92);
    ///
    /// assert_eq!(Lazy::force(&lazy), &92);
    /// assert_eq!(&*lazy, &92);
    /// ```
    pub fn force(this: &Self) -> &T {
        this.cell.call_once(|| match this.init.take() {
            Some(f) => f(),
            None => panic!("Lazy instance has previously been poisoned"),
        })
    }
}

impl<T, F: FnOnce() -> T> Deref for Lazy<T, F> {
    type Target = T;

    fn deref(&self) -> &T {
        Self::force(self)
    }
}

impl<T: Default> Default for Lazy<T, fn() -> T> {
    /// Creates a new lazy value using `Default` as the initializing function.
    fn default() -> Self {
        Self::new(T::default)
    }
}

use {
    crate::once::Once,
    core::{cell::Cell, ops::Deref},
};

pub struct Lazy<T, F = fn() -> T> {
    cell: Once<T>,
    init: Cell<Option<F>>,
}

unsafe impl<T, F: Send> Sync for Lazy<T, F> where Once<T>: Sync {}

impl<T, F> Lazy<T, F> {
    /* TODO */
    #[inline(always)]
    pub const fn new(f: F) -> Lazy<T, F> {
        Lazy {
            cell: Once::new(),
            init: Cell::new(Some(f)),
        }
    }
}

impl<T, F: FnOnce() -> T> Lazy<T, F> {
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

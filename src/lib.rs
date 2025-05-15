#![cfg_attr(not(test), no_std)]

extern crate alloc;

#[cfg(feature = "lazy")]
pub mod lazy;

#[cfg(feature = "lazy")]
pub use lazy::Lazy;

#[cfg(feature = "mutex")]
pub mod mutex;

#[cfg(feature = "mutex")]
pub use mutex::{Mutex, MutexGuard};

#[cfg(feature = "once")]
pub mod once;

#[cfg(feature = "once")]
pub use once::Once;

#[cfg(feature = "rwlock")]
pub mod rwlock;

#[cfg(feature = "rwlock")]
pub use rwlock::{RwLock, RwLockReadGuard, RwLockUpgradableGuard, RwLockWriteGuard};

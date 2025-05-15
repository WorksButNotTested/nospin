#![cfg_attr(not(test), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

//! This crate provides non thread-safe versions of the primitives in `std::sync` and `std::lazy`
//! suitable for use in single-threaded `no_std` environments.
//!
//! # Features
//!
//! - `Mutex`, `RwLock`, `Once`/`SyncOnceCell`, and `SyncLazy` equivalents
//!
//! - Support for `no_std` environments
//!
//! - [`lock_api`](https://crates.io/crates/lock_api) compatibility
//!
//! - Upgradeable `RwLock` guards
//!
//! - Guards can be sent and shared between threads
//!
//! - Guard leaking
//!
//! # Relationship with `std::sync`
//!
//! While `spin` is not a drop-in replacement for `std::sync` an effort has been
//! made to keep this crate reasonably consistent with `std::sync`.
//!
//! ## Feature flags
//!
//! The crate comes with a few feature flags that you may wish to use.
//!
//! - `lock_api` enables support for [`lock_api`](https://crates.io/crates/lock_api)
extern crate alloc;

#[cfg(feature = "lazy")]
#[cfg_attr(docsrs, doc(cfg(feature = "lazy")))]
pub mod lazy;

#[cfg(feature = "lazy")]
pub use lazy::Lazy;

#[cfg(feature = "mutex")]
#[cfg_attr(docsrs, doc(cfg(feature = "mutex")))]
pub mod mutex;

#[cfg(feature = "mutex")]
pub use mutex::{Mutex, MutexGuard};

#[cfg(feature = "once")]
#[cfg_attr(docsrs, doc(cfg(feature = "once")))]
pub mod once;

#[cfg(feature = "once")]
pub use once::Once;

#[cfg(feature = "rwlock")]
#[cfg_attr(docsrs, doc(cfg(feature = "rwlock")))]
pub mod rwlock;

#[cfg(feature = "rwlock")]
pub use rwlock::{RwLock, RwLockReadGuard, RwLockUpgradableGuard, RwLockWriteGuard};

/// Spin synchronisation primitives, but compatible with [`lock_api`](https://crates.io/crates/lock_api).
#[cfg(feature = "lock_api")]
#[cfg_attr(docsrs, doc(cfg(feature = "lock_api")))]
pub mod lock_api {
    /// A lock that provides mutually exclusive data access (compatible with [`lock_api`](https://crates.io/crates/lock_api)).
    #[cfg(feature = "mutex")]
    #[cfg_attr(docsrs, doc(cfg(feature = "mutex")))]
    pub type Mutex<T> = lock_api_crate::Mutex<crate::Mutex<()>, T>;

    /// A guard that provides mutable data access (compatible with [`lock_api`](https://crates.io/crates/lock_api)).
    #[cfg(feature = "mutex")]
    #[cfg_attr(docsrs, doc(cfg(feature = "mutex")))]
    pub type MutexGuard<'a, T> = lock_api_crate::MutexGuard<'a, crate::Mutex<()>, T>;

    /// A lock that provides data access to either one writer or many readers (compatible with [`lock_api`](https://crates.io/crates/lock_api)).
    #[cfg(feature = "rwlock")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rwlock")))]
    pub type RwLock<T> = lock_api_crate::RwLock<crate::RwLock<()>, T>;

    /// A guard that provides immutable data access (compatible with [`lock_api`](https://crates.io/crates/lock_api)).
    #[cfg(feature = "rwlock")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rwlock")))]
    pub type RwLockReadGuard<'a, T> = lock_api_crate::RwLockReadGuard<'a, crate::RwLock<()>, T>;

    /// A guard that provides mutable data access (compatible with [`lock_api`](https://crates.io/crates/lock_api)).
    #[cfg(feature = "rwlock")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rwlock")))]
    pub type RwLockWriteGuard<'a, T> = lock_api_crate::RwLockWriteGuard<'a, crate::RwLock<()>, T>;

    /// A guard that provides immutable data access but can be upgraded to [`RwLockWriteGuard`] (compatible with [`lock_api`](https://crates.io/crates/lock_api)).
    #[cfg(feature = "rwlock")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rwlock")))]
    pub type RwLockUpgradableReadGuard<'a, T> =
        lock_api_crate::RwLockUpgradableReadGuard<'a, crate::RwLock<()>, T>;

    /// A guard returned by [RwLockReadGuard::map] that provides immutable data access (compatible with [`lock_api`](https://crates.io/crates/lock_api)).
    #[cfg(feature = "rwlock")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rwlock")))]
    pub type MappedRwLockReadGuard<'a, T> =
        lock_api_crate::MappedRwLockReadGuard<'a, crate::RwLock<()>, T>;

    /// A guard returned by [RwLockWriteGuard::map] that provides mutable data access (compatible with [`lock_api`](https://crates.io/crates/lock_api)).
    #[cfg(feature = "rwlock")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rwlock")))]
    pub type MappedRwLockWriteGuard<'a, T> =
        lock_api_crate::MappedRwLockWriteGuard<'a, crate::RwLock<()>, T>;
}

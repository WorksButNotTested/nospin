#![no_std]

#[cfg(feature = "lazy")]
pub mod lazy;

#[cfg(feature = "lazy")]
pub use lazy::Lazy;

#[cfg(feature = "mutex")]
pub mod mutex;

#[cfg(feature = "mutex")]
pub use mutex::Mutex;

#[cfg(feature = "once")]
pub mod once;

#[cfg(feature = "once")]
pub use once::Once;

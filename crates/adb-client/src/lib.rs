//! ADB client library for Android device communication.

mod adb;
mod error;

pub use adb::Adb;
pub use error::AdbError;

//! Utility methods for writing and executing tests
//!
//! Provides the [`TestResult`] type in [`result`]. And a custom [`catch_unwind`].

use crate::{TestError, TestResult};
use anyhow::anyhow;
use std::panic::UnwindSafe;

pub mod result;

/// Run the closure passed and turn any panic into [`TestError::Error`].
pub fn catch_unwind<T: FnOnce() -> TestResult + UnwindSafe>(f: T) -> TestResult {
    match std::panic::catch_unwind(f) {
        Ok(Ok(())) => Ok(()),
        Ok(e @ Err(_)) => e,
        Err(panic) => {
            if let Some(msg) = panic.downcast_ref::<&str>() {
                Err(TestError::Error(anyhow!("Test panicked: {msg:?}")))
            } else if let Some(msg) = panic.downcast_ref::<String>() {
                Err(TestError::Error(anyhow!("Test panicked: {msg:?}")))
            } else {
                Err(TestError::Error(anyhow!("Test panicked with a weird type")))
            }
        }
    }
}

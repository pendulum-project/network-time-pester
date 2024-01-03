use crate::{TestError, TestResult};
use anyhow::anyhow;
use std::panic::UnwindSafe;

pub mod result;

pub fn catch_unwind<T: FnOnce() -> TestResult + UnwindSafe>(f: T) -> TestResult {
    match std::panic::catch_unwind(f) {
        Ok(Ok(())) => Ok(()),
        Ok(e @ Err(_)) => return e,
        Err(panic) => {
            if let Some(msg) = panic.downcast_ref::<&str>() {
                return Err(TestError::Error(anyhow!("Test panicked: {msg:?}")));
            }
            if let Some(msg) = panic.downcast_ref::<String>() {
                return Err(TestError::Error(anyhow!("Test panicked: {msg:?}")));
            }
            return Err(TestError::Error(anyhow!("Test panicked with a weird type")));
        }
    }
}

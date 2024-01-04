//! Definitions for a custom error type that can indicate if a test failed or an error occurred

use crate::Response;
use std::error::Error;
use std::fmt::{Display, Formatter};

/// Convenience wrapper for a [`Result`] around a [`TestError`].
pub type TestResult<T = ()> = Result<T, TestError>;

/// The negative outcome of a test
#[derive(Debug)]
pub enum TestError {
    /// The test failed, meaning the impl under test did something wrong
    Fail(String, Option<Box<Response>>),
    /// The test was skipped e.g. because NTS was not available
    Skipped,
    /// An error occurred, this could be caused by the impl under test, or something else
    Error(anyhow::Error),
}

impl From<anyhow::Error> for TestError {
    fn from(value: anyhow::Error) -> Self {
        Self::Error(value)
    }
}

impl Display for TestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TestError::Fail(msg, _) => {
                write!(f, "Test case failed: {msg}")
            }
            TestError::Skipped => {
                write!(f, "Test was skipped")
            }
            TestError::Error(e) => {
                write!(f, "A different error occurred: {e}")
            }
        }
    }
}

impl Error for TestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            TestError::Error(e) => Some(e.as_ref()),
            TestError::Fail(_, _) | TestError::Skipped => None,
        }
    }
}

/// Convenience const to pass a test
pub const PASS: TestResult<()> = Ok(());

/// Construct a [`TestError::Fail`] instance with response data
pub fn fail<T>(msg: impl ToString, response: impl Into<Response>) -> TestResult<T> {
    Err(TestError::Fail(
        msg.to_string(),
        Some(Box::new(response.into())),
    ))
}

/// Construct a [`TestError::Fail`] instance without response data
pub fn fail_no_response<T>(msg: impl ToString) -> TestResult<T> {
    Err(TestError::Fail(msg.to_string(), None))
}

use crate::Response;
use std::error::Error;
use std::fmt::{Display, Formatter};

pub type TestResult<T = ()> = Result<T, TestError>;

#[derive(Debug)]
pub enum TestError {
    Fail(String, Option<Box<Response>>),
    Skipped,
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

pub const PASS: TestResult<()> = Ok(());

pub fn fail<T>(msg: impl ToString, response: impl Into<Response>) -> TestResult<T> {
    Err(TestError::Fail(
        msg.to_string(),
        Some(Box::new(response.into())),
    ))
}

pub fn fail_no_response<T>(msg: impl ToString) -> TestResult<T> {
    Err(TestError::Fail(msg.to_string(), None))
}

pub fn expected_response() -> TestError {
    TestError::Fail("Did not receive a reply".to_string(), None)
}

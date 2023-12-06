use crate::Response;

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

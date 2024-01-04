macro_rules! pester_assert {
    ($response:expr, $cond:expr $(,)?) => {
        let condition = $cond;
        if !condition {
            return crate::fail(concat!("Assertion ", stringify!($cond), " failed"), $response);
        }
    };
    ($response:expr, $cond:expr, $($arg:tt)+) => {
        let condition = $cond;
        if !condition {
            return crate::fail(format!("Assertion {} failed: {}", stringify!($cond), format!($($arg)+)), $response);
        }
    };
}
pub(crate) use pester_assert;

macro_rules! pester_assert_eq {
    ($response:expr, $actual:expr, $expected:expr $(,)?) => {
        let a = $expected;
        let b = $actual;
        if a != b {
            return crate::fail(format!("Assertion {} equal to {} failed, expected {:?}, actual {:?}", stringify!($actual), stringify!($expected), a, b), $response);
        }
    };
    ($response:expr, $actual:expr, $expected:expr, $($arg:tt)+) => {
        let a = $expected;
        let b = $actual;
        if a != b {
            return crate::fail(format!("Assertion {} equal to {} failed, expected {:?}, actual {:?}: {}", stringify!($actual), stringify!($expected), a, b, format!($($arg)+)), $response);
        }
    };
}
pub(crate) use pester_assert_eq;

#[allow(unused)]
macro_rules! pester_assert_ne {
    ($response:expr, $actual:expr, $expected:expr $(,)?) => {
        let a = $expected;
        let b = $actual;
        if a == b {
            return crate::fail(format!("Assertion {} not equal to {} failed, value {:?}", stringify!($actual), stringify!($expected), a), $response);
        }
    };
    ($response:expr, $actual:expr, $expected:expr, $($arg:tt)+) => {
        let a = $expected;
        let b = $actual;
        if a == b {
            return crate::fail(format!("Assertion {} not equal to {} failed, value {:?}: {}", stringify!($actual), stringify!($expected), a, format!($($arg)+)), $response);
        }
    };
}
#[allow(unused)]
pub(crate) use pester_assert_ne;

macro_rules! pester_assert_gt {
    ($response:expr, $actual:expr, $expected:expr $(,)?) => {
        let a = $expected;
        let b = $actual;
        if b <= a {
            return crate::fail(format!("Assertion {} greater than {} failed, value {:?}, bound {:?}", stringify!($actual), stringify!($expected), b, a), $response);
        }
    };
    ($response:expr, $actual:expr, $expected:expr, $($arg:tt)+) => {
        let a = $expected;
        let b = $actual;
        if b <= a {
            return crate::fail(format!("Assertion {} greater than {} failed, value {:?}, bound {:?}: {}", stringify!($actual), stringify!($expected), b, a, format!($($arg)+)), $response);
        }
    };
}
pub(crate) use pester_assert_gt;

macro_rules! pester_assert_lt {
    ($response:expr, $actual:expr, $expected:expr $(,)?) => {
        let a = $expected;
        let b = $actual;
        if b >= a {
            return crate::fail(format!("Assertion {} smaller than {} failed, value {:?}, bound {:?}", stringify!($actual), stringify!($expected), b, a), $response);
        }
    };
    ($response:expr, $actual:expr, $expected:expr, $($arg:tt)+) => {
        let a = $expected;
        let b = $actual;
        if b >= a {
            return crate::fail(format!("Assertion {} smaller than {} failed, value {:?}, bound {:?}: {}", stringify!($actual), stringify!($expected), b, a, format!($($arg)+)), $response);
        }
    };
}
pub(crate) use pester_assert_lt;

macro_rules! pester_assert_response {
    ($response:expr $(,)?) => {
        if let Some(r) = $response {
            r
        } else {
            return crate::fail_no_response();
        }
    };
}
pub(crate) use pester_assert_response;

macro_rules! pester_assert_no_response {
    ($response:expr $(,)?) => {
        if let Some(r) = $response {
            return crate::fail("Unexpected response from server", r);
        }
    };
    ($response:expr, $($arg:tt)+) => {
        if let Some(r) = $response {
            return crate::fail(format!("Unexpected response from server: {}", format!($($arg)+)), r);
        }
    };
}
pub(crate) use pester_assert_no_response;

macro_rules! pester_assert_parsable {
    ($response:expr $(,)?) => {
        match ntp_proto::NtpPacket::try_from(&$response) {
            Ok(packet) => packet,
            Err(e) => return crate::fail(format!("Could not parse response: {e}"), $response),
        }
    };
}
pub(crate) use pester_assert_parsable;

macro_rules! pester_assert_version {
    ($response:expr, $packet:expr, $version:ident $(,)?) => {
        if let ntp_proto::NtpHeader::$version(h) = $packet.header() {
            h
        } else {
            return crate::fail(
                format!(
                    "Server replied with version {} instead of {}",
                    $packet.version(),
                    stringify!($version),
                ),
                $response,
            );
        }
    };
}
pub(crate) use pester_assert_version;

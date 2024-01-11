//! Functions and types for implementing NTS tests

use crate::udp::{udp_server_still_alive, UdpConnection};
use crate::{RawBytes, TestCase, TestConfig, TestResult};
use ntp_proto::NtsKeys;
use std::ops::Deref;
use std::panic::UnwindSafe;

/// A wrapper for a NTS cookie
///
/// Using this wrapper ensures we can not mix up byte slices and improves debug printing.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NtsCookie(pub RawBytes);

impl Deref for NtsCookie {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0 .0
    }
}

/// Wrap a test function that requires a server connection, as well as NTS data
///
/// The function passed will be called during test execution. It gets passed a [`UdpConnection`] to the target server,
/// as well as a valid NTS server cookie, and matching key set.
///
/// If the test completes successfully this wrapper checks if the server still replies to normal requests.
pub fn nts_test<F>(f: F) -> Box<dyn TestCase + UnwindSafe>
where
    F: Fn(&mut UdpConnection, NtsCookie, &NtsKeys) -> TestResult + UnwindSafe + 'static,
{
    struct KeTest<F> {
        f: F,
    }

    impl<F> TestCase for KeTest<F>
    where
        F: Fn(&mut UdpConnection, NtsCookie, &NtsKeys) -> TestResult + 'static,
    {
        fn name(&self) -> &'static str {
            std::any::type_name::<F>()
        }

        fn run(&self, conf: &TestConfig) -> TestResult {
            let mut conn = conf.udp()?;
            let (test_cookie, keys) = conf.take_cookie()?;
            (self.f)(&mut conn, test_cookie, &keys)?;

            udp_server_still_alive(&mut conn, Some(conf.take_cookie()?))
        }
    }

    Box::new(KeTest { f })
}

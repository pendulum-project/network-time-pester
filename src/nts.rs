use crate::udp::{udp_server_still_alive, UdpConnection};
use crate::{RawBytes, TestCase, TestConfig, TestResult};
use ntp_proto::NtsKeys;
use std::ops::Deref;
use std::panic::UnwindSafe;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NtsCookie(pub RawBytes);

impl Deref for NtsCookie {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0 .0
    }
}

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

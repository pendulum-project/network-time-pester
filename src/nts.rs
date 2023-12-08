use crate::nts_ke::NtsKeConnection;
use crate::udp::{udp_server_still_alive, UdpConnection};
use crate::util::result::{fail, fail_no_response, PASS};
use crate::{NtsServer, RawBytes, TestCase, TestConfig, TestResult};
use anyhow::Context;
use ntp_proto::{NtpPacket, PollInterval};
use std::net::{SocketAddr, ToSocketAddrs};
use std::ops::Deref;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NtsCookie(pub RawBytes);

impl Deref for NtsCookie {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0 .0
    }
}

pub fn nts_test<F>(f: F) -> Box<dyn TestCase>
where
    F: Fn(&mut UdpConnection, NtsCookie) -> TestResult + 'static,
{
    struct KeTest<F> {
        f: F,
    }

    impl<F> TestCase for KeTest<F>
    where
        F: Fn(&mut UdpConnection, NtsCookie) -> TestResult + 'static,
    {
        fn name(&self) -> &'static str {
            std::any::type_name::<F>()
        }

        fn run(&self, conf: &TestConfig) -> TestResult {
            let mut conn = conf.udp()?;
            let [test_cookie, still_alive_cookie] = conf.take_cookie_pair()?;
            (self.f)(&mut conn, test_cookie)?;

            udp_server_still_alive(&mut conn, Some(still_alive_cookie))
        }
    }

    Box::new(KeTest { f })
}

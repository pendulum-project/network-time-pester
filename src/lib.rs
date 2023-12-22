use anyhow::Context;
use ntp_proto::{NoCipher, NtpPacket, PacketParsingError};
use std::fmt::{Debug, Formatter};
use std::io::{Cursor, ErrorKind};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;

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

macro_rules! pester_assert_response {
    ($response:expr $(,)?) => {
        if let Some(r) = $response {
            r
        } else {
            return crate::fail_no_response();
        }
    };
}

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

macro_rules! pester_assert_parsable {
    ($response:expr $(,)?) => {
        match ntp_proto::NtpPacket::try_from(&$response) {
            Ok(packet) => packet,
            Err(e) => return crate::fail(format!("Could not parse response: {e}"), $response),
        }
    };
}

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

macro_rules! pester_assert_server_responsive {
    ($conn:expr $(,)?) => {
        if !$conn
            .pester(ntp_proto::NtpPacket::poll_message(core::default::Default::default()).0)?
            .is_some()
        {
            return crate::fail_no_response();
        }
    };
}

// This allows us to generate nice docs around our tests while we still get
// warnings for unused test cases
#[cfg(doc)]
pub mod tests;
#[cfg(not(doc))]
mod tests;

pub use tests::all_tests;

pub struct Connection {
    socket: UdpSocket,
}

pub struct Request(pub Vec<u8>);

impl From<NtpPacket<'_>> for Request {
    fn from(value: NtpPacket) -> Self {
        let mut buffer = vec![0u8; Connection::MAX_LEN];
        let mut cursor = Cursor::new(buffer.as_mut_slice());

        value
            .serialize(&mut cursor, &NoCipher, None)
            .expect("Serializing into a vec can not fail");

        let length = cursor.position() as usize;

        buffer.truncate(length);

        Self(buffer)
    }
}

impl From<&[u8]> for Request {
    fn from(value: &[u8]) -> Self {
        Self(value.into())
    }
}

impl<const N: usize> From<[u8;N]> for Request {
    fn from(value: [u8;N]) -> Self {
        Self(value.into())
    }
}

impl<const N: usize> From<&[u8;N]> for Request {
    fn from(value: &[u8;N]) -> Self {
        Self(value.into())
    }
}

#[derive(Clone)]
pub struct Response(pub Vec<u8>);

impl<'a> TryFrom<&'a Response> for NtpPacket<'a> {
    type Error = PacketParsingError<'a>;

    fn try_from(value: &'a Response) -> Result<Self, Self::Error> {
        let (packet, _cookie) = NtpPacket::deserialize(value.0.as_slice(), &NoCipher)?;

        Ok(packet)
    }
}

impl Debug for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Response")
            .field("parsed", &NtpPacket::try_from(self))
            .field("raw", &hex::encode(self.0.as_slice()))
            .finish()
    }
}

impl Connection {
    const MAX_LEN: usize = 9000;
    const TIMEOUT: Duration = Duration::from_millis(100);

    pub fn new(to_addr: impl ToSocketAddrs) -> anyhow::Result<Self> {
        let mut to_addr = to_addr
            .to_socket_addrs()
            .context("Could not parse peer address")?;
        let to_addr = to_addr
            .next()
            .context("Domain did not resolve into any addresses")?;

        let from_addr: SocketAddr = match to_addr {
            SocketAddr::V4(_) => "0.0.0.0:0",
            SocketAddr::V6(_) => "[::0]:0",
        }
        .parse()
        .expect("no errors where made writing this address");

        let socket = UdpSocket::bind(from_addr).context("Could not open socket")?;
        socket
            .connect(to_addr)
            .with_context(|| format!("Can not connect to {to_addr} from {from_addr}"))?;
        socket
            .set_read_timeout(Some(Self::TIMEOUT))
            .context("Could not set timeout")?;

        Ok(Self { socket })
    }

    pub fn pester(&mut self, req: impl Into<Request>) -> anyhow::Result<Option<Response>> {
        self.socket
            .send(req.into().0.as_slice())
            .context("Could not send request")?;

        let mut response = vec![0; Self::MAX_LEN];
        let len = match self.socket.recv(response.as_mut_slice()) {
            Ok(len) => len,
            Err(err) => match err.kind() {
                ErrorKind::TimedOut | ErrorKind::WouldBlock => return Ok(None),
                _ => return Err(err).context("Could not receive response"),
            },
        };
        response.truncate(len);

        Ok(Some(Response(response)))
    }
}

pub trait TestCase {
    fn name(&self) -> &'static str;
    fn run(&self, conn: &mut Connection) -> anyhow::Result<TestResult>;
}

impl<F> TestCase for F
where
    F: Fn(&mut Connection) -> anyhow::Result<TestResult>,
{
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn run(&self, conn: &mut Connection) -> anyhow::Result<TestResult> {
        self(conn)
    }
}

pub enum TestResult {
    Pass,
    Fail(String, Option<Response>),
}

const PASS: anyhow::Result<TestResult> = Ok(TestResult::Pass);

fn fail(msg: impl ToString, response: Response) -> anyhow::Result<TestResult> {
    Ok(TestResult::Fail(msg.to_string(), Some(response)))
}

fn fail_no_response() -> anyhow::Result<TestResult> {
    Ok(TestResult::Fail("Server did not respond".to_string(), None))
}

use anyhow::{bail, Context};
use ntp_proto::{NoCipher, NtpPacket, PacketParsingError};
use std::io::{Cursor, ErrorKind};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;

pub mod tests;

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

pub struct Response(pub Vec<u8>);

impl<'a> TryFrom<&'a Response> for NtpPacket<'a> {
    type Error = PacketParsingError<'a>;

    fn try_from(value: &'a Response) -> Result<Self, Self::Error> {
        let (packet, _cookie) = NtpPacket::deserialize(value.0.as_slice(), &NoCipher)?;

        Ok(packet)
    }
}

impl Connection {
    const MAX_LEN: usize = 9000;
    const TIMEOUT: Duration = Duration::from_millis(100);

    pub fn new(to_addr: impl ToSocketAddrs) -> anyhow::Result<Self> {
        let mut to_addr = to_addr.to_socket_addrs().context("parsing peer address")?;
        let to_addr = match (to_addr.next(), to_addr.next()) {
            (Some(addr), None) => addr,
            _ => bail!("wrong number of peer addresses, should be exactly one"),
        };

        let from_addr: SocketAddr = match to_addr {
            SocketAddr::V4(_) => "127.0.0.1:0",
            SocketAddr::V6(_) => "[::1]:0",
        }
        .parse()
        .expect("no errors where made writing this address");

        let socket = UdpSocket::bind(from_addr).context("Opening socket")?;
        socket
            .connect(to_addr)
            .with_context(|| format!("Connect to {to_addr}"))?;
        socket
            .set_read_timeout(Some(Self::TIMEOUT))
            .context("Setting timeout")?;

        Ok(Self { socket })
    }

    pub fn pester(&mut self, req: impl Into<Request>) -> anyhow::Result<Option<Response>> {
        self.socket
            .send(req.into().0.as_slice())
            .context("Sending request")?;

        let mut response = vec![0; Self::MAX_LEN];
        let len = match self.socket.recv(response.as_mut_slice()) {
            Ok(len) => len,
            Err(err) => match err.kind() {
                ErrorKind::TimedOut | ErrorKind::WouldBlock => return Ok(None),
                _ => return Err(err).context("Receiving response"),
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
    Fail,
}

const PASS: anyhow::Result<TestResult> = Ok(TestResult::Pass);
const FAIL: anyhow::Result<TestResult> = Ok(TestResult::Fail);

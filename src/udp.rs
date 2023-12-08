use crate::nts::NtsCookie;
use crate::util::result::{fail, fail_no_response, TestResult, PASS};
use crate::{Response, TestCase, TestConfig, TestError};
use anyhow::Context;
use ntp_proto::{NoCipher, NtpPacket, PacketParsingError, PollInterval};
use std::fmt::{Debug, Formatter};
use std::io::{Cursor, ErrorKind};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;

pub struct UdpConnection {
    socket: UdpSocket,
}

pub struct UdpRequest(pub Vec<u8>);

impl From<NtpPacket<'_>> for UdpRequest {
    fn from(value: NtpPacket) -> Self {
        let mut buffer = vec![0u8; UdpConnection::MAX_LEN];
        let mut cursor = Cursor::new(buffer.as_mut_slice());

        value
            .serialize(&mut cursor, &NoCipher, None)
            .expect("Serializing into a vec can not fail");

        let length = cursor.position() as usize;

        buffer.truncate(length);

        Self(buffer)
    }
}

#[derive(Clone)]
pub struct UdpResponse(pub Vec<u8>);

impl<'a> TryFrom<&'a UdpResponse> for NtpPacket<'a> {
    type Error = PacketParsingError<'a>;

    fn try_from(value: &'a UdpResponse) -> Result<Self, Self::Error> {
        let (packet, _cookie) = NtpPacket::deserialize(value.0.as_slice(), &NoCipher)?;

        Ok(packet)
    }
}

impl Debug for UdpResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Response")
            .field("parsed", &NtpPacket::try_from(self))
            .field("raw", &hex::encode(self.0.as_slice()))
            .finish()
    }
}

impl UdpConnection {
    const MAX_LEN: usize = 9000;

    pub fn new(to_addr: impl ToSocketAddrs, timeout: Duration) -> TestResult<Self> {
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
            .set_read_timeout(Some(timeout))
            .context("Could not set timeout")?;

        Ok(Self { socket })
    }

    pub fn pester(&mut self, req: impl Into<UdpRequest>) -> TestResult<Option<NtpPacket>> {
        self.socket
            .send(req.into().0.as_slice())
            .context("Could not send request")?;

        let mut response = vec![0; Self::MAX_LEN];
        let len = match self.socket.recv(response.as_mut_slice()) {
            Ok(len) => len,
            Err(err) => match err.kind() {
                ErrorKind::TimedOut | ErrorKind::WouldBlock => return Ok(None),
                _ => Err(err).context("Could not receive response")?,
            },
        };
        response.truncate(len);

        let packet = match NtpPacket::deserialize(response.as_slice(), &NoCipher) {
            Ok((packet, _cookie)) => packet,
            Err(e) => {
                return Err(TestError::Fail(
                    format!("Server replied with invalid packet: {e:?}"),
                    Some(Box::new(Response::UdpUnparsable(response.clone().into()))),
                ))
            }
        };

        Ok(Some(packet.into_owned()))
    }
}

pub fn udp_test<F>(f: F) -> Box<dyn TestCase>
where
    F: Fn(&mut UdpConnection) -> TestResult + 'static,
{
    struct UdpTest<F> {
        f: F,
    }

    impl<F> TestCase for UdpTest<F>
    where
        F: Fn(&mut UdpConnection) -> TestResult,
    {
        fn name(&self) -> &'static str {
            std::any::type_name::<F>()
        }

        fn run(&self, conf: &TestConfig) -> TestResult {
            let mut conn = conf.udp()?;
            (self.f)(&mut conn)?;

            udp_server_still_alive(&mut conn, None)
        }
    }

    Box::new(UdpTest { f })
}

pub fn udp_server_still_alive(conn: &mut UdpConnection, cookie: Option<NtsCookie>) -> TestResult {
    // Check that we did not kill the server
    let (req, id) = match cookie {
        None => NtpPacket::poll_message(PollInterval::default()),
        Some(ref cookie) => NtpPacket::nts_poll_message(cookie, 1, PollInterval::default()),
    };
    match conn.pester(req) {
        Ok(Some(response)) if response.valid_server_response(id, cookie.is_some()) => PASS,
        Ok(Some(response)) => fail(
            "After test: Poll was answered by invalid response",
            response,
        ),
        Ok(None) => fail_no_response("After test: Server did no longer reply to normal poll"),
        Err(e) => fail_no_response(format!(
            "After test: Server did no longer reply to normal poll. Error: {e:?}"
        )),
    }
}

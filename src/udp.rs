use crate::nts::NtsCookie;
use crate::util::result::{fail, fail_no_response, TestResult, PASS};
use crate::{TestCase, TestConfig};
use anyhow::Context;
use ntp_proto::{NoCipher, NtpPacket, NtsKeys, PacketParsingError, PollInterval};
use std::fmt::{Debug, Formatter};
use std::io::{Cursor, ErrorKind};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::panic::UnwindSafe;
use std::sync::Arc;
use std::time::Duration;

pub struct UdpConnection {
    socket: UdpSocket,
}

pub struct UdpRequest(pub Vec<u8>);

impl UdpRequest {
    pub fn from_ntp_packet(packet: NtpPacket, keys: Option<&NtsKeys>) -> Self {
        let mut buffer = vec![0u8; UdpConnection::MAX_LEN];
        let mut cursor = Cursor::new(buffer.as_mut_slice());

        packet
            .serialize(&mut cursor, &keys.map(|k| k.c2s.as_ref()), None)
            .expect("Serializing into a vec can not fail");

        let length = cursor.position() as usize;

        buffer.truncate(length);

        Self(buffer)
    }
}

impl From<NtpPacket<'_>> for UdpRequest {
    fn from(value: NtpPacket) -> Self {
        Self::from_ntp_packet(value, None)
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

    pub fn pester_raw(&mut self, req: UdpRequest) -> TestResult<Option<UdpResponse>> {
        self.socket
            .send(req.0.as_slice())
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

        Ok(Some(UdpResponse(response)))
    }

    fn pester_pkt(
        &mut self,
        packet: NtpPacket,
        keys: Option<&NtsKeys>,
    ) -> TestResult<Option<NtpPacket>> {
        let req = UdpRequest::from_ntp_packet(packet, keys);
        let response = match self.pester_raw(req)? {
            None => return Ok(None),
            Some(r) => r,
        };

        let packet =
            match NtpPacket::deserialize(response.0.as_slice(), &keys.map(|k| k.s2c.as_ref())) {
                Ok((packet, _cookie)) => packet,
                Err(e) => {
                    return fail(
                        format!("Server replied with invalid packet: {e:?}"),
                        response,
                    )
                }
            };

        Ok(Some(packet.into_owned()))
    }

    pub fn pester(&mut self, packet: NtpPacket) -> TestResult<Option<NtpPacket>> {
        self.pester_pkt(packet, None)
    }

    pub fn pester_nts(
        &mut self,
        packet: NtpPacket,
        keys: &NtsKeys,
    ) -> TestResult<Option<NtpPacket>> {
        self.pester_pkt(packet, Some(keys))
    }
}

pub fn udp_test<F>(f: F) -> Box<dyn TestCase + UnwindSafe>
where
    F: Fn(&mut UdpConnection) -> TestResult + UnwindSafe + 'static,
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

pub fn udp_server_still_alive(
    conn: &mut UdpConnection,
    nts: Option<(NtsCookie, Arc<NtsKeys>)>,
) -> TestResult {
    // Check that we did not kill the server
    let (req, id) = match &nts {
        None => NtpPacket::poll_message(PollInterval::default()),
        Some((cookie, _)) => NtpPacket::nts_poll_message(cookie, 1, PollInterval::default()),
    };

    let result = match &nts {
        None => conn.pester(req),
        Some((_, keys)) => conn.pester_nts(req, keys),
    };

    match result {
        Ok(Some(response)) if response.valid_server_response(id, nts.is_some()) => PASS,
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

//! Functionality to connect to a NTS-KE server and run tests against it

use crate::nts::NtsCookie;
use crate::util::result::{fail, TestError, TestResult};
use crate::{TestCase, TestConfig};
use anyhow::{anyhow, Context};
use ntp_proto::{AeadAlgorithm, AesSivCmac256, NtsKeys, NtsRecord, NtsRecordDecoder, ProtocolId};
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use std::fmt::Debug;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::panic::UnwindSafe;
use std::sync::Arc;
use std::time::Duration;

/// An active connection to a NTS-KE server
///
/// This allows to send and receive records one at a time using [`send_record`](NtsKeConnection::send_record), and
/// [`recv_record`](NtsKeConnection::recv_record). Or run a whole exchange using
/// [`exchange`](NtsKeConnection::exchange), or [`do_request`](NtsKeConnection::do_request).
pub struct NtsKeConnection {
    stream: StreamOwned<ClientConnection, TcpStream>,
    host: String,
    record_decoder: NtsRecordDecoder,
}

impl NtsKeConnection {
    /// Connect to the server given by `host` and `port`
    ///
    /// The `root_cert_store` is used to verify the server signature
    pub fn new(
        host: &str,
        port: u16,
        root_cert_store: &Arc<RootCertStore>,
        timeout: Duration,
    ) -> TestResult<Self> {
        let addr = (host, port)
            .to_socket_addrs()
            .context(format!("Could not resolve host: {host:?}"))?
            .next()
            .context(format!("Host has no IP entries: {host:?}"))?;

        let mut config = ClientConfig::builder()
            .with_root_certificates(Arc::clone(root_cert_store))
            .with_no_client_auth();

        // Ensure we send only ntske/1 as alpn
        config.alpn_protocols.clear();
        config.alpn_protocols.push(b"ntske/1".to_vec());

        let domain = ServerName::try_from(host)
            .context("invalid dnsname")?
            .to_owned();
        let connection = ClientConnection::new(Arc::new(config), domain)
            .context("Could not open TLS connection")?;
        let stream = TcpStream::connect(addr).context("Could not open TCP connection")?;
        stream
            .set_read_timeout(Some(timeout))
            .context("Could not set read timeout")?;
        stream
            .set_write_timeout(Some(timeout))
            .context("Could not set write timeout")?;

        let stream = StreamOwned::new(connection, stream);

        Ok(Self {
            stream,
            host: host.to_string(),
            record_decoder: Default::default(),
        })
    }

    /// Serialize and send a single record to the server
    pub fn send_record(&mut self, record: NtsRecord) -> TestResult {
        let mut buf = vec![];
        record
            .write(&mut buf)
            .expect("Writing into a vec can not fail");

        self.stream
            .write_all(&buf[..])
            .context("Failed to write to TLS connection")?;

        Ok(())
    }

    /// Try to receive the next record from the server
    ///
    /// Behaves similar to an iterator. Returns `Ok(Some(record))` until all records have been received when it returns
    /// `Ok(None)`.
    pub fn recv_record(&mut self) -> TestResult<Option<NtsRecord>> {
        loop {
            if let Some(record) = self
                .record_decoder
                .step()
                .context("Could not read from NTS records")?
            {
                return Ok(Some(record));
            }

            let mut buf = vec![0; 1024];
            let read_bytes = self
                .stream
                .read(&mut buf)
                .context("Could not read from TLS connection")?;
            buf.truncate(read_bytes);
            if buf.is_empty() {
                return Ok(None);
            }

            self.record_decoder.extend(buf);
        }
    }

    /// Perform a complete exchange with the server
    ///
    /// This sends all the records provided by `request` in one go and then parses the response and returns it.
    pub fn exchange(
        &mut self,
        request: impl IntoIterator<Item = NtsRecord>,
    ) -> TestResult<Response> {
        let mut buf = vec![];
        for rec in request {
            rec.write(&mut buf).expect("Vec never runs out of space");
        }
        self.stream.write_all(&buf).context("Failed to write TLS")?;

        let mut records = vec![];
        loop {
            let last = records.last();
            match self.recv_record() {
                Ok(Some(rec)) => records.push(rec),
                Ok(None) if last == Some(&NtsRecord::EndOfMessage) => break,
                Ok(None) => {
                    return fail(
                        "NTS-KE closed connection without sending EndOfMessage",
                        records,
                    )
                }
                Err(e) => Err(anyhow!(e).context("Could not read next record"))?,
            }
        }

        let response = Response::try_from(records)?;
        Ok(response)
    }

    /// Perform a complete request/response cycle with default data, extracting all data needed to contact the UDP side.
    pub fn do_request(&mut self) -> TestResult<(Vec<NtsCookie>, SocketAddr, NtsKeys)> {
        let response = self.exchange([
            NtsRecord::NextProtocol {
                protocol_ids: vec![ProtocolId::NtpV4 as u16],
            },
            NtsRecord::AeadAlgorithm {
                critical: false,
                algorithm_ids: vec![AeadAlgorithm::AeadAesSivCmac256 as u16],
            },
            NtsRecord::EndOfMessage,
        ])?;

        let Some(&[aead]) = response.aead.as_deref() else {
            return fail("KE did not reply with exactly one AEAD", response);
        };
        let aead = AeadAlgorithm::try_deserialize(aead).context("invalid AEAD")?;
        if aead != AeadAlgorithm::AeadAesSivCmac256 {
            return fail("KE replied with an aead we did not ask for", response);
        }

        let Some(&[next_protocol]) = response.next_protocol.as_deref() else {
            return fail("KE did not reply with exactly one next_protocol", response);
        };
        let next_protocol =
            ProtocolId::try_deserialize(next_protocol).context("invalid next protocol")?;
        if next_protocol != ProtocolId::NtpV4 {
            return fail("KE replied with an protocol we did not ask for", response);
        }

        // TODO: Once ntp-proto updated rustls: Use AeadAlgorithm::extract_nts_keys directly
        let c2s = extract_nts_key(&self.stream.conn, aead.c2s_context(ProtocolId::NtpV4))
            .context("Could not extract session keys")?;
        let s2c = extract_nts_key(&self.stream.conn, aead.s2c_context(ProtocolId::NtpV4))
            .context("Could not extract session keys")?;

        let c2s = Box::new(AesSivCmac256::new(c2s));
        let s2c = Box::new(AesSivCmac256::new(s2c));

        let keys = NtsKeys { c2s, s2c };

        let host = response.server.as_deref().unwrap_or(&self.host);
        let port = response.port.unwrap_or(123);

        let udp_host = format!("{host}:{port}")
            .to_socket_addrs()
            .with_context(|| format!("Could not resolve {host}:{port}"))?
            .next()
            .with_context(|| format!("{host:?} did not resolve into any IPs"))?;

        Ok((response.cookies, udp_host, keys))
    }
}

fn extract_nts_key<T: Default + AsMut<[u8]>, ConnectionData>(
    tls_connection: &rustls::ConnectionCommon<ConnectionData>,
    context: [u8; 5],
) -> Result<T, rustls::Error> {
    let mut key = T::default();
    tls_connection.export_keying_material(
        &mut key,
        b"EXPORTER-network-time-security",
        Some(context.as_slice()),
    )?;

    Ok(key)
}

/// Wrap a function taking a fresh connection to a NTS-KE server, turning it into a [`TestCase`].
pub fn ke_test<F>(f: F) -> Box<dyn TestCase + UnwindSafe>
where
    F: Fn(&mut NtsKeConnection) -> TestResult + UnwindSafe + 'static,
{
    struct KeTest<F> {
        f: F,
    }

    impl<F> TestCase for KeTest<F>
    where
        F: Fn(&mut NtsKeConnection) -> TestResult,
    {
        fn name(&self) -> &'static str {
            std::any::type_name::<F>()
        }

        fn run(&self, conf: &TestConfig) -> TestResult {
            let mut conn = conf.ke()?;
            (self.f)(&mut conn)
        }
    }

    Box::new(KeTest { f })
}

/// Convenience wrapper around all fields needed for a NTS-KE request
#[derive(Clone, Eq, PartialEq)]
pub struct Request {
    pub next_protocol: Vec<u16>,
    pub aead: Vec<u16>,
    pub critical_aead: bool,
    pub server: Option<String>,
    pub port: Option<u16>,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            next_protocol: vec![0],
            aead: vec![15],
            critical_aead: false,
            server: None,
            port: None,
        }
    }
}

impl IntoIterator for Request {
    type Item = NtsRecord;
    type IntoIter = std::vec::IntoIter<NtsRecord>;

    fn into_iter(self) -> Self::IntoIter {
        let mut recs = vec![];

        recs.push(NtsRecord::NextProtocol {
            protocol_ids: self.next_protocol,
        });
        recs.push(NtsRecord::AeadAlgorithm {
            critical: self.critical_aead,
            algorithm_ids: self.aead,
        });
        if let Some(server) = self.server {
            recs.push(NtsRecord::Server {
                critical: false,
                name: server,
            });
        }
        if let Some(port) = self.port {
            recs.push(NtsRecord::Port {
                critical: false,
                port,
            });
        }

        // TODO shuffle records here

        recs.push(NtsRecord::EndOfMessage);

        recs.into_iter()
    }
}

/// Convenience wrapper around all fields that can be contained in a NTS-KE response.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Response {
    pub next_protocol: Option<Vec<u16>>,
    pub errors: Vec<u16>,
    pub warnings: Vec<u16>,
    pub aead: Option<Vec<u16>>,
    pub cookies: Vec<NtsCookie>,
    pub server: Option<String>,
    pub port: Option<u16>,
}

impl TryFrom<Vec<NtsRecord>> for Response {
    type Error = TestError;

    fn try_from(records: Vec<NtsRecord>) -> Result<Self, Self::Error> {
        let mut next_protocol = None;
        let mut errors = vec![];
        let mut warnings = vec![];
        let mut aead = None;
        let mut cookies = vec![];
        let mut server = None;
        let mut port = None;

        let mut iter_records = records.clone();
        let last = iter_records.pop();
        if last != Some(NtsRecord::EndOfMessage) {
            return fail(
                format!("Response did not end in EndOfMessage instead ended in: {last:?}"),
                records,
            );
        }

        for rec in iter_records {
            match rec {
                NtsRecord::NextProtocol { protocol_ids } => {
                    if next_protocol.replace(protocol_ids).is_some() {
                        return fail(
                            "Response included more then one NTS Next Protocol Negotiation record",
                            records,
                        );
                    }
                }
                NtsRecord::Error { errorcode } => errors.push(errorcode),
                NtsRecord::Warning { warningcode } => warnings.push(warningcode),
                NtsRecord::AeadAlgorithm {
                    critical: _,
                    algorithm_ids,
                } => {
                    if aead.replace(algorithm_ids).is_some() {
                        return fail(
                            "Response included more then one AEAD Algorithm Negotiation record",
                            records,
                        );
                    }
                }
                NtsRecord::NewCookie { cookie_data } => cookies.push(NtsCookie(cookie_data.into())),
                NtsRecord::Server { critical: _, name } => {
                    if server.replace(name).is_some() {
                        return fail(
                            "Response included more then one NTPv4 Server Negotiation record",
                            records,
                        );
                    }
                }
                NtsRecord::Port {
                    critical: _,
                    port: rec_port,
                } => {
                    if port.replace(rec_port).is_some() {
                        return fail(
                            "Response included more then one NTPv4 Port Negotiation record",
                            records,
                        );
                    }
                }
                other => {
                    return fail(
                        format!("Response included an unexpected field: {other:?}"),
                        records,
                    )
                }
            }
        }

        Ok(Self {
            next_protocol,
            errors,
            warnings,
            aead,
            cookies,
            server,
            port,
        })
    }
}

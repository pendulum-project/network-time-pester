// This allows us to generate nice docs around our tests while we still get
// warnings for unused test cases
#[cfg(doc)]
pub mod tests;
#[cfg(not(doc))]
mod tests;

pub mod nts;
pub mod nts_ke;
pub mod udp;
pub mod util;

use crate::nts_ke::NtsKeConnection;
use anyhow::anyhow;
use ntp_proto::{NtsKeys, NtsRecord};
use rustls::RootCertStore;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::nts::NtsCookie;
pub use tests::all_tests;
pub use util::result::{TestError, TestResult};

#[derive(Debug)]
pub struct NtsServer {
    host: String,
    port: u16,
    root_cert_store: Arc<RootCertStore>,
    udp_host: SocketAddr,
    nts: Mutex<(Vec<NtsCookie>, Arc<NtsKeys>)>,
    timeout: Duration,
}

impl NtsServer {
    pub fn new(
        host: String,
        port: u16,
        ca_file: Option<PathBuf>,
        timeout: Duration,
    ) -> TestResult<Self> {
        let root_cert_store = root_ca(ca_file)?;

        let mut ke = NtsKeConnection::new(&host, port, &root_cert_store, timeout)?;
        let (cookies, udp_host, keys) = ke.do_request()?;

        Ok(Self {
            host,
            port,
            root_cert_store,
            udp_host,
            nts: Mutex::new((cookies, Arc::new(keys))),
            timeout,
        })
    }

    pub fn udp_host(&self) -> SocketAddr {
        self.udp_host
    }

    pub fn take_cookie(&self) -> TestResult<(NtsCookie, Arc<NtsKeys>)> {
        let mut guard = self.nts.lock().expect("No poisoned cookies");

        if guard.0.is_empty() {
            self.refill(&mut guard)?;
        }

        Ok((
            guard.0.pop().expect("Just refilled the jar"),
            Arc::clone(&guard.1),
        ))
    }

    fn refill(&self, (cookies, keys): &mut (Vec<NtsCookie>, Arc<NtsKeys>)) -> TestResult {
        assert!(cookies.is_empty());

        let mut ke =
            NtsKeConnection::new(&self.host, self.port, &self.root_cert_store, self.timeout)?;
        let (new_cookies, udp_host, new_keys) = ke.do_request()?;

        if udp_host != self.udp_host {
            return Err(TestError::Error(anyhow!(
                "Server switched to which UDP host it points"
            )));
        }

        cookies.extend(new_cookies);
        let _old_keys = std::mem::replace(keys, Arc::new(new_keys));

        Ok(())
    }
}

#[derive(Debug)]
pub enum Server {
    Ntp(SocketAddr),
    Nts(NtsServer),
}

#[derive(Debug)]
pub struct TestConfig {
    pub server: Server,
    pub timeout: Duration,
}

impl TestConfig {
    pub fn udp(&self) -> TestResult<udp::UdpConnection> {
        let addr = match &self.server {
            Server::Ntp(addr) => *addr,
            Server::Nts(server) => server.udp_host(),
        };

        udp::UdpConnection::new(addr, self.timeout)
    }

    pub fn ke(&self) -> TestResult<NtsKeConnection> {
        match &self.server {
            Server::Ntp(_) => Err(TestError::Skipped),
            Server::Nts(server) => NtsKeConnection::new(
                &server.host,
                server.port,
                &server.root_cert_store,
                server.timeout,
            ),
        }
    }

    pub fn take_cookie(&self) -> TestResult<(NtsCookie, Arc<NtsKeys>)> {
        let Server::Nts(server) = &self.server else {
            return Err(TestError::Skipped);
        };

        server.take_cookie()
    }
}

pub fn root_ca(cafile: Option<PathBuf>) -> anyhow::Result<Arc<RootCertStore>> {
    let mut root_cert_store = RootCertStore::empty();
    if let Some(cafile) = &cafile {
        let mut pem = BufReader::new(File::open(cafile)?);
        for cert in rustls_pemfile::certs(&mut pem) {
            root_cert_store.add(cert?).unwrap();
        }
    } else {
        root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    }

    Ok(Arc::new(root_cert_store))
}

#[derive(Clone, Eq, PartialEq)]
pub struct RawBytes(pub Box<[u8]>);

impl Debug for RawBytes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        hex::encode(&self.0).fmt(f)
    }
}

impl From<Vec<u8>> for RawBytes {
    fn from(value: Vec<u8>) -> Self {
        Self(value.into_boxed_slice())
    }
}

#[derive(Debug, Clone)]
pub enum Response {
    UdpUnparsable(RawBytes),
    UdpResponse(ntp_proto::NtpPacket<'static>),
    KeResponse(nts_ke::Response),
    KeInvalid(Vec<NtsRecord>),
}

impl From<udp::UdpResponse> for Response {
    fn from(value: udp::UdpResponse) -> Self {
        Self::UdpUnparsable(value.0.into())
    }
}

impl<'a> From<ntp_proto::NtpPacket<'a>> for Response {
    fn from(value: ntp_proto::NtpPacket<'a>) -> Self {
        Self::UdpResponse(value.into_owned())
    }
}

impl From<nts_ke::Response> for Response {
    fn from(value: nts_ke::Response) -> Self {
        Self::KeResponse(value)
    }
}

impl From<Vec<NtsRecord>> for Response {
    fn from(value: Vec<NtsRecord>) -> Self {
        Self::KeInvalid(value)
    }
}

pub trait TestCase {
    fn name(&self) -> &'static str;
    fn run(&self, conn: &TestConfig) -> TestResult;
}

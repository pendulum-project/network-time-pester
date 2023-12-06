// This allows us to generate nice docs around our tests while we still get
// warnings for unused test cases
#[cfg(doc)]
pub mod tests;
#[cfg(not(doc))]
mod tests;

pub mod nts_ke;
pub mod udp;
pub mod util;

use crate::nts_ke::NtsKeConnection;
use ntp_proto::NtsRecord;
use rustls::RootCertStore;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub use tests::all_tests;
pub use util::result::{TestError, TestResult};

pub struct TestConfig {
    pub udp: Option<SocketAddr>,
    pub ke: Option<(String, u16)>,
    pub root_cert_store: Arc<RootCertStore>,
    pub timeout: Duration,
}

impl TestConfig {
    pub fn udp(&self) -> TestResult<udp::UdpConnection> {
        match self.udp {
            Some(addr) => udp::UdpConnection::new(addr, self.timeout),
            None => Err(TestError::Skipped),
        }
    }

    pub fn ke(&self) -> TestResult<NtsKeConnection> {
        match self.ke {
            Some((ref host, port)) => {
                NtsKeConnection::new(host, port, Arc::clone(&self.root_cert_store), self.timeout)
            }
            None => Err(TestError::Skipped),
        }
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

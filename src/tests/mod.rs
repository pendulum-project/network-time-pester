use crate::{Connection, TestCase, TestResult, FAIL, PASS};
use ntp_proto::NtpPacket;

pub fn tests() -> Vec<Box<dyn TestCase>> {
    vec![
        Box::new(test_ignores_version_5),
        Box::new(test_responds_to_version_4),
    ]
}

pub fn test_responds_to_version_4(conn: &mut Connection) -> anyhow::Result<TestResult> {
    let (packet, _id) = NtpPacket::poll_message(Default::default());
    let response = conn.pester(packet)?;

    // TODO check response

    match response {
        None => FAIL,
        Some(_) => PASS,
    }
}

pub fn test_ignores_version_5(conn: &mut Connection) -> anyhow::Result<TestResult> {
    let (packet, _id) = NtpPacket::poll_message_v5(Default::default());
    let response = conn.pester(packet)?;

    match response {
        None => PASS,
        Some(_) => FAIL,
    }
}

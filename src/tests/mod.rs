use crate::{Connection, TestCase, TestResult, FAIL, PASS};
use ntp_proto::NtpPacket;

/// Generate a list of all currently implemented test cases
pub fn all_tests() -> Vec<Box<dyn TestCase>> {
    vec![
        Box::new(test_ignores_version_5),
        Box::new(test_responds_to_version_4),
    ]
}

/// Sending a normal poll request should return an answer
///
/// Checks that the tested server actually responds to us.
pub fn test_responds_to_version_4(conn: &mut Connection) -> anyhow::Result<TestResult> {
    let (packet, _id) = NtpPacket::poll_message(Default::default());
    let response = conn.pester(packet)?;

    // TODO check response

    match response {
        None => FAIL,
        Some(_) => PASS,
    }
}

/// Check that unknown versions are ignore
///
/// Since NTPv5 is not released yet any compliant server should still ignore
/// packets with this version number.
pub fn test_ignores_version_5(conn: &mut Connection) -> anyhow::Result<TestResult> {
    let (packet, _id) = NtpPacket::poll_message_v5(Default::default());
    let response = conn.pester(packet)?;

    match response {
        None => PASS,
        Some(_) => FAIL,
    }
}

use crate::{macros::*, Connection, TestResult, PASS};
use ntp_proto::{NtpAssociationMode, NtpPacket};

/// Sending a normal poll request should return an answer
///
/// Checks that the tested server actually responds to us.
pub fn test_responds_to_version_4(conn: &mut Connection) -> anyhow::Result<TestResult> {
    let (request, id) = NtpPacket::poll_message(Default::default());
    let response = conn.pester(request)?;

    let response = pester_assert_response!(response);
    let packet = pester_assert_parsable!(response);
    let header = pester_assert_version!(response, packet, V4);

    pester_assert_eq!(
        response,
        header.origin_timestamp,
        id.expected_origin_timestamp,
        "Incorrect origin timestamp"
    );
    pester_assert!(
        response,
        packet.valid_server_response(id, false),
        "Server response not matching original packet"
    );

    pester_assert_gt!(
        response,
        header.transmit_timestamp,
        header.receive_timestamp,
        "Receive should happen before send of response"
    );
    pester_assert_eq!(
        response,
        header.mode,
        NtpAssociationMode::Server,
        "Incorrect mode in server response"
    );

    PASS
}

/// Check that unknown versions are ignore
///
/// Since NTPv5 is not released yet any compliant server should still ignore
/// packets with this version number.
pub fn test_ignores_version_5(conn: &mut Connection) -> anyhow::Result<TestResult> {
    let (packet, _id) = NtpPacket::poll_message_v5(Default::default());
    let response = conn.pester(packet)?;

    pester_assert_no_response!(response, "Should not respond to ntp version 5 requests");

    PASS
}

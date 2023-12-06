use crate::udp::UdpConnection;
use crate::util::result::{expected_response, fail, TestResult, PASS};
use ntp_proto::{NtpAssociationMode, NtpHeader, NtpPacket};

/// Sending a normal poll request should return an answer
///
/// Checks that the tested server actually responds to us.
pub fn test_responds_to_version_4(conn: &mut UdpConnection) -> TestResult {
    let (request, id) = NtpPacket::poll_message(Default::default());
    let response = conn.pester(request)?.ok_or_else(expected_response)?;

    let NtpHeader::V4(header) = response.header() else {
        return fail(
            format!(
                "Server replied with version {} instead of 4",
                response.version()
            ),
            response,
        );
    };

    if header.origin_timestamp != id.expected_origin_timestamp {
        return fail(
            format!(
                "Server replied with incorrect origin timestamp. Should have been {:?}, was {:?}",
                id.expected_origin_timestamp, header.origin_timestamp
            ),
            response,
        );
    }
    assert!(response.valid_server_response(id, false));

    if header.receive_timestamp > header.transmit_timestamp {
        return fail(
            "Server claims to have received the packet after sending the reply",
            response,
        );
    }

    if header.mode != NtpAssociationMode::Server {
        return fail(
            format!(
                "Server replied with incorrect mode: {:?} should have been {:?}",
                header.mode,
                NtpAssociationMode::Server
            ),
            response,
        );
    }

    PASS
}

/// Check that unknown versions are ignore
///
/// Since NTPv5 is not released yet any compliant server should still ignore
/// packets with this version number.
pub fn test_ignores_version_5(conn: &mut UdpConnection) -> TestResult {
    let (packet, _id) = NtpPacket::poll_message_v5(Default::default());
    let response = conn.pester(packet)?;

    match response {
        None => PASS,
        Some(r) => fail(
            "Server did respond to NTPv5 request, should have ignored",
            r,
        ),
    }
}

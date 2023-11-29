use crate::{fail, fail_no_response, Connection, TestCase, TestResult, PASS};
use anyhow::bail;
use ntp_proto::{ExtensionField, NtpAssociationMode, NtpHeader, NtpPacket};
use std::array;
use std::borrow::Cow;

/// Generate a list of all currently implemented test cases
pub fn all_tests() -> Vec<Box<dyn TestCase>> {
    vec![
        Box::new(test_responds_to_version_4),
        Box::new(test_ignores_version_5),
        Box::new(test_unknown_extensions_are_ignored),
        Box::new(test_unique_id_is_returned),
    ]
}

/// Sending a normal poll request should return an answer
///
/// Checks that the tested server actually responds to us.
pub fn test_responds_to_version_4(conn: &mut Connection) -> anyhow::Result<TestResult> {
    let (request, id) = NtpPacket::poll_message(Default::default());
    let response = conn.pester(request)?;

    let Some(response) = response else {
        return fail_no_response();
    };

    let packet = match NtpPacket::try_from(&response) {
        Ok(packet) => packet,
        Err(e) => return fail(format!("Could not parse response: {e}"), response),
    };

    let NtpHeader::V4(header) = packet.header() else {
        return fail(
            format!(
                "Server replied with version {} instead of 4",
                packet.version()
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
    assert!(packet.valid_server_response(id, false));

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
pub fn test_ignores_version_5(conn: &mut Connection) -> anyhow::Result<TestResult> {
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

/// Test if a server ignores invalid extensions
///
/// A NTP server should ignore any extension fields which it can not handle.
pub fn test_unknown_extensions_are_ignored(conn: &mut Connection) -> anyhow::Result<TestResult> {
    let (mut request, id) = NtpPacket::poll_message(Default::default());
    request.push_additional(ExtensionField::Unknown {
        type_id: 0,
        data: Cow::Borrowed(&[]),
    });

    let Some(response) = conn.pester(request)? else {
        return fail_no_response();
    };

    let packet = match NtpPacket::try_from(&response) {
        Ok(packet) => packet,
        Err(e) => return fail(format!("Could not parse response: {e}"), response),
    };

    if !packet.valid_server_response(id, false) {
        return fail("Server replied with wrong id", response);
    }

    if packet.authenticated_extension_fields().next().is_some() {
        bail!("Parsed an authenticated extension from a non NTS packet, this is a bug!");
    }

    if let Some(ef) = packet.untrusted_extension_fields().next() {
        return fail(format!("Received an extension field in response to an invalid extension field. (EF: {ef:?})"), response.clone());
    }

    PASS
}

/// Test if a server returned a unique id field as is even without NTS
///
/// A server supporting NTS should still reply with the unique id extension that
/// the client sent.
pub fn test_unique_id_is_returned(conn: &mut Connection) -> anyhow::Result<TestResult> {
    let (mut request, id) = NtpPacket::poll_message(Default::default());
    let uid = ExtensionField::UniqueIdentifier(Cow::Owned(
        array::from_fn::<_, 32, _>(|i| i as u8).to_vec(),
    ));
    request.push_additional(uid.clone());

    let Some(response) = conn.pester(request)? else {
        return fail_no_response();
    };

    let packet = match NtpPacket::try_from(&response) {
        Ok(packet) => packet,
        Err(e) => return fail(format!("Could not parse response: {e}"), response),
    };

    if !packet.valid_server_response(id, false) {
        return fail("Server replied with wrong id", response);
    }

    if packet.authenticated_extension_fields().next().is_some() {
        bail!("Parsed an authenticated extension from a non NTS packet, this is a bug!");
    }

    let fields: Vec<_> = packet.untrusted_extension_fields().collect();
    if fields.is_empty() {
        return fail(
            "Server did not send a unique id extension field in its reply.",
            response,
        );
    }

    if fields.len() >= 2 {
        return fail(
            format!("Server returned more then one extension fields. (Fields: {fields:?})"),
            response,
        );
    }

    if fields[0] != &uid {
        return fail(
            "The unique id of the response does not match the one in the request!",
            response,
        );
    }

    PASS
}

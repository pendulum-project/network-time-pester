use crate::{fail, Connection, TestCase, TestResult, PASS};
use anyhow::bail;
use ntp_proto::{ExtensionField, NtpAssociationMode, NtpPacket};
use std::array;
use std::borrow::Cow;

/// Generate a list of all currently implemented test cases
pub fn all_tests() -> Vec<Box<dyn TestCase>> {
    vec![
        Box::new(test_responds_to_version_4),
        Box::new(test_ignores_version_5),
        Box::new(test_unknown_extensions_are_ignored),
        Box::new(test_unique_id_is_returned),
        Box::new(test_odd_sized_packets),
        Box::new(test_invalid_extension_fields),
    ]
}

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
    let response = conn.pester(b"\x2B\x02\x06\xe9\x00\x00\x02\x36\x00\x00\x03\xb7\xc0\x35\x67\x6c\xe5\xf6\x61\xfd\x6f\x16\x5f\x03\xe5\xf6\x63\xa8\x76\x19\xef\x40\xe5\xf6\x63\xa8\x79\x8c\x65\x81\xe5\xf6\x63\xa8\x79\x8e\xae\x2b")?;

    pester_assert_no_response!(response, "Should not respond to ntp version 5 requests");

    pester_assert_server_responsive!(conn);

    PASS
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

    let response = pester_assert_response!(conn.pester(request)?);
    let packet = pester_assert_parsable!(response);

    pester_assert!(
        response,
        packet.valid_server_response(id, false),
        "Server response not matching original packet"
    );

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

    let response = pester_assert_response!(conn.pester(request)?);
    let packet = pester_assert_parsable!(response);

    pester_assert!(
        response,
        packet.valid_server_response(id, false),
        "Server response not matching original packet"
    );

    if packet.authenticated_extension_fields().next().is_some() {
        bail!("Parsed an authenticated extension from a non NTS packet, this is a bug!");
    }

    let fields: Vec<_> = packet.untrusted_extension_fields().collect();
    pester_assert!(
        response,
        !fields.is_empty(),
        "Server dit not reply with unique id EF"
    );
    pester_assert_lt!(
        response,
        fields.len(),
        2,
        "Too many extension fields provided by server (Fields: {:?})",
        fields
    );

    pester_assert_eq!(
        response,
        fields[0],
        &uid,
        "Response UID does not match request"
    );

    PASS
}

fn test_odd_sized_packets(conn: &mut Connection) -> anyhow::Result<TestResult> {
    let base = b"\x23\x02\x06\xe8\x00\x00\x03\xff\x00\x00\x03\x7d\x5e\xc6\x9f\x0f\xe5\xf6\x62\x98\x7b\x61\xb9\xaf\xe5\xf6\x63\x66\x7b\x64\x99\x5d\xe5\xf6\x63\x66\x81\x40\x55\x90\xe5\xf6\x63\xa8\x76\x1d\xde\x48\x01\x02\x03\x04\x05";

    for i in 0..=base.len() {
        let response = conn.pester(&base[..i])?;
        if let Some(response) = response {
            pester_assert_parsable!(response);
        }
        pester_assert_server_responsive!(conn);
    }

    PASS
}

fn test_invalid_extension_fields(conn: &mut Connection) -> anyhow::Result<TestResult> {
    let response = conn.pester(b"\x23\x02\x06\xe8\x00\x00\x03\xff\x00\x00\x03\x7d\x5e\xc6\x9f\x0f\xe5\xf6\x62\x98\x7b\x61\xb9\xaf\xe5\xf6\x63\x66\x7b\x64\x99\x5d\xe5\xf6\x63\x66\x81\x40\x55\x90\xe5\xf6\x63\xa8\x76\x1d\xde\x48\xab\xcd\xf0\xf05678901234567890123456789012")?;
    if let Some(response) = response {
        pester_assert_parsable!(response);
    }
    pester_assert_server_responsive!(conn);

    let response = conn.pester(b"\x23\x02\x06\xe8\x00\x00\x03\xff\x00\x00\x03\x7d\x5e\xc6\x9f\x0f\xe5\xf6\x62\x98\x7b\x61\xb9\xaf\xe5\xf6\x63\x66\x7b\x64\x99\x5d\xe5\xf6\x63\x66\x81\x40\x55\x90\xe5\xf6\x63\xa8\x76\x1d\xde\x48\xab\xcd\x00\x055678901234567890123456789012")?;
    if let Some(response) = response {
        pester_assert_parsable!(response);
    }
    pester_assert_server_responsive!(conn);

    let response = conn.pester(b"\x23\x02\x06\xe8\x00\x00\x03\xff\x00\x00\x03\x7d\x5e\xc6\x9f\x0f\xe5\xf6\x62\x98\x7b\x61\xb9\xaf\xe5\xf6\x63\x66\x7b\x64\x99\x5d\xe5\xf6\x63\x66\x81\x40\x55\x90\xe5\xf6\x63\xa8\x76\x1d\xde\x48\xab\xcd\x00\x065678901234567890123456789012")?;
    if let Some(response) = response {
        pester_assert_parsable!(response);
    }
    pester_assert_server_responsive!(conn);

    let response = conn.pester(b"\x23\x02\x06\xe8\x00\x00\x03\xff\x00\x00\x03\x7d\x5e\xc6\x9f\x0f\xe5\xf6\x62\x98\x7b\x61\xb9\xaf\xe5\xf6\x63\x66\x7b\x64\x99\x5d\xe5\xf6\x63\x66\x81\x40\x55\x90\xe5\xf6\x63\xa8\x76\x1d\xde\x48\xab\xcd\x00\x075678901234567890123456789012")?;
    if let Some(response) = response {
        pester_assert_parsable!(response);
    }
    pester_assert_server_responsive!(conn);

    PASS
}

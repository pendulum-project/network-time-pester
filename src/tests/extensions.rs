use crate::{fail, macros::*, Connection, TestResult, PASS};
use anyhow::bail;
use ntp_proto::{ExtensionField, NtpPacket};
use std::borrow::Cow;
use std::{array, format};

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

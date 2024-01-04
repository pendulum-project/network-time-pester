//! Tests that exercise the extension field mechanism described in [RFC5905 section 7.5](https://datatracker.ietf.org/doc/html/rfc5905#section-7.5)

use crate::macros::*;
use crate::udp::UdpConnection;
use crate::util::result::{fail, TestResult, PASS};
use anyhow::anyhow;
use ntp_proto::{ExtensionField, NtpPacket};
use std::borrow::Cow;
use std::{array, format};

/// Test if a server ignores invalid extensions
///
/// A NTP server should ignore any extension fields which it can not handle.
pub fn test_unknown_extensions_are_ignored(conn: &mut UdpConnection) -> TestResult {
    let (mut request, id) = NtpPacket::poll_message(Default::default());
    request.push_additional(ExtensionField::Unknown {
        type_id: 0,
        data: Cow::Borrowed(&[]),
    });

    let packet = pester_assert_response!(conn.pester(request)?);

    pester_assert!(
        packet,
        packet.valid_server_response(id, false),
        "Server response not matching original packet"
    );

    if packet.authenticated_extension_fields().next().is_some() {
        Err(anyhow!(
            "Parsed an authenticated extension from a non NTS packet, this is a bug!"
        ))?;
    }

    if let Some(ef) = packet.untrusted_extension_fields().next() {
        return fail(format!("Received an extension field in response to an invalid extension field. (EF: {ef:?})"), packet.clone());
    }

    PASS
}

/// Test if a server returned a unique id field as is even without NTS
///
/// A server supporting NTS should still reply with the unique id extension that
/// the client sent.
pub fn test_unique_id_is_returned(conn: &mut UdpConnection) -> TestResult {
    let (mut request, id) = NtpPacket::poll_message(Default::default());
    let uid = ExtensionField::UniqueIdentifier(Cow::Owned(
        array::from_fn::<_, 32, _>(|i| i as u8).to_vec(),
    ));
    request.push_additional(uid.clone());

    let packet = pester_assert_response!(conn.pester(request)?);

    pester_assert!(
        packet,
        packet.valid_server_response(id, false),
        "Server response not matching original packet"
    );

    if packet.authenticated_extension_fields().next().is_some() {
        Err(anyhow!(
            "Parsed an authenticated extension from a non NTS packet, this is a bug!"
        ))?;
    }

    let fields: Vec<_> = packet.untrusted_extension_fields().collect();
    pester_assert!(
        packet,
        !fields.is_empty(),
        "Server dit not reply with unique id EF"
    );
    pester_assert_lt!(
        packet,
        fields.len(),
        2,
        "Too many extension fields provided by server (Fields: {:?})",
        fields
    );

    pester_assert_eq!(
        packet,
        fields[0],
        &uid,
        "Response UID does not match request"
    );

    PASS
}

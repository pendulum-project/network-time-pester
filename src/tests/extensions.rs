use crate::udp::UdpConnection;
use crate::util::result::{expected_response, fail, TestResult, PASS};
use anyhow::anyhow;
use ntp_proto::{ExtensionField, NtpPacket};
use std::array;
use std::borrow::Cow;

/// Test if a server ignores invalid extensions
///
/// A NTP server should ignore any extension fields which it can not handle.
pub fn test_unknown_extensions_are_ignored(conn: &mut UdpConnection) -> TestResult {
    let (mut request, id) = NtpPacket::poll_message(Default::default());
    request.push_additional(ExtensionField::Unknown {
        type_id: 0,
        data: Cow::Borrowed(&[]),
    });

    let response = conn.pester(request)?.ok_or_else(expected_response)?;

    if !response.valid_server_response(id, false) {
        return fail("Server replied with wrong id", response);
    }

    if response.authenticated_extension_fields().next().is_some() {
        Err(anyhow!(
            "Parsed an authenticated extension from a non NTS packet, this is a bug!"
        ))?;
    }

    if let Some(ef) = response.untrusted_extension_fields().next() {
        return fail(format!("Received an extension field in response to an invalid extension field. (EF: {ef:?})"), response.clone());
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

    let response = conn.pester(request)?.ok_or_else(expected_response)?;

    if !response.valid_server_response(id, false) {
        return fail("Server replied with wrong id", response);
    }

    if response.authenticated_extension_fields().next().is_some() {
        Err(anyhow!(
            "Parsed an authenticated extension from a non NTS packet, this is a bug!"
        ))?;
    }

    let fields: Vec<_> = response.untrusted_extension_fields().collect();
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

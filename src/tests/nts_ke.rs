//! Test that exercise the NTS Key Establishment Protocol (NTS-KE)
//!
//! The protocol is specified in [RFC8915 section 4](https://datatracker.ietf.org/doc/html/rfc8915#name-the-nts-key-establishment-p).

use crate::macros::{pester_assert, pester_assert_eq};
use crate::nts_ke::{NtsKeConnection, Request};
use crate::util::result::{TestResult, PASS};
use ntp_proto::NtsRecord;

/// Check that the server responds with a valid response to a valid request
pub fn happy(ke: &mut NtsKeConnection) -> TestResult {
    let res = ke.exchange(Request::default())?;

    pester_assert_eq!(
        res,
        res.next_protocol.clone(),
        Some(vec![0]),
        "Server did reply with different protocols then we asked for",
    );

    pester_assert_eq!(
        res,
        res.aead.clone(),
        Some(vec![15]),
        "Server did reply with different AEAD then we asked for"
    );

    pester_assert!(
        res,
        res.errors.is_empty(),
        "Server did reply with error code to normal request",
    );

    pester_assert!(
        res,
        res.warnings.is_empty(),
        "Server did reply with warning code to normal request",
    );

    pester_assert_eq!(
        res,
        res.cookies.len(),
        8,
        "Server did not reply with 8 cookies"
    );

    PASS
}

/// Check that the server replies with an empty protocol list if we send only protocols that do not exist
///
/// See also [ignore_unknown_extra_protocols]
pub fn error_on_unknown_next_protocol(ke: &mut NtsKeConnection) -> TestResult {
    let request = Request {
        next_protocol: vec![0xFFFF], // A reserved next protocol id
        ..Request::default()
    };
    let response = ke.exchange(request)?;

    pester_assert_eq!(
        response,
        response.next_protocol.clone(),
        Some(vec![]),
        "Server did not respond with empty next protocol"
    );

    PASS
}

/// Check that the server ignores unknown protocols
///
/// See [RFC8915 section 4.1.2](https://datatracker.ietf.org/doc/html/rfc8915#section-4.1.2)
pub fn ignore_unknown_extra_protocols(ke: &mut NtsKeConnection) -> TestResult {
    let request = Request {
        next_protocol: vec![0xFFFF, 0], // A reserved next protocol id
        ..Request::default()
    };
    let response = ke.exchange(request)?;

    pester_assert_eq!(
        response,
        response.next_protocol.clone(),
        Some(vec![0]),
        "Server did not respond with expected next protocol"
    );

    PASS
}

/// Check that the server replies with an empty AEAD list if we send only algorithms that do not exist
///
/// See also [ignore_unknown_extra_aead]
pub fn error_on_unknown_aead(ke: &mut NtsKeConnection) -> TestResult {
    let response = ke.exchange(Request {
        aead: vec![0xFFFF],
        ..Request::default()
    })?;

    pester_assert_eq!(
        response,
        response.aead.clone(),
        Some(vec![]),
        "Server did not respond with empty aead"
    );

    PASS
}

/// Check that the server ignores unknown AEAD algorithms
///
/// See [RFC8915 section 4.1.5](https://datatracker.ietf.org/doc/html/rfc8915#name-aead-algorithm-negotiation)
pub fn ignore_unknown_extra_aead(ke: &mut NtsKeConnection) -> TestResult {
    let response = ke.exchange(Request {
        aead: vec![0xFFFF, 15], // 0xFFFF is reserved for private use
        ..Request::default()
    })?;

    pester_assert_eq!(
        response,
        response.aead.clone(),
        Some(vec![15]),
        "Server did not respond with expected aead"
    );

    PASS
}

/// Check that the server replies with an error message even to an invalid request
///
/// See [RFC8915 section 4.1.3](https://datatracker.ietf.org/doc/html/rfc8915#name-error)
pub fn empty_message_resolves_in_error(ke: &mut NtsKeConnection) -> TestResult {
    let response = ke.exchange([NtsRecord::EndOfMessage])?;

    pester_assert_eq!(
        response,
        response.errors.clone(),
        vec![1],
        "Server did not respond with error to empty message"
    );

    PASS
}

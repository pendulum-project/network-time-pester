use crate::nts_ke::{NtsKeConnection, Request};
use crate::util::result::{fail, TestResult, PASS};
use ntp_proto::NtsRecord;

pub fn happy(ke: &mut NtsKeConnection) -> TestResult {
    let response = ke.exchange(Request::default())?;

    if response.next_protocol != Some(vec![0]) {
        return fail(
            format!(
                "Server did reply with different protocols then we asked for: {:?} expected [0]",
                response.next_protocol
            ),
            response,
        );
    }

    if response.aead != Some(vec![15]) {
        return fail(
            format!(
                "Server did reply with different AEAD then we asked for: {:?} expected [15]",
                response.next_protocol
            ),
            response,
        );
    }

    if !response.errors.is_empty() {
        return fail(
            "Server did reply with error code to normal request",
            response,
        );
    }

    if !response.warnings.is_empty() {
        return fail(
            "Server did reply with warning code to normal request",
            response,
        );
    }

    if response.cookies.len() != 8 {
        return fail("Server did not reply with 8 cookies", response);
    }

    PASS
}

pub fn error_on_unknown_next_protocol(ke: &mut NtsKeConnection) -> TestResult {
    let request = Request {
        next_protocol: vec![0xFFFF], // A reserved next protocol id
        ..Request::default()
    };
    let response = ke.exchange(request)?;

    match response.next_protocol.as_deref() {
        Some(&[]) => PASS,
        _ => fail("Server did not respond with empty next protocol", response),
    }
}

pub fn ignore_unknown_extra_protocols(ke: &mut NtsKeConnection) -> TestResult {
    let request = Request {
        next_protocol: vec![0xFFFF, 0], // A reserved next protocol id
        ..Request::default()
    };
    let response = ke.exchange(request)?;

    match response.next_protocol.as_deref() {
        Some(&[0]) => PASS,
        _ => fail(
            "Server did not respond with expected next protocol",
            response,
        ),
    }
}

pub fn error_on_unknown_aead(ke: &mut NtsKeConnection) -> TestResult {
    let response = ke.exchange(Request {
        aead: vec![0xFFFF],
        ..Request::default()
    })?;

    match response.aead.as_deref() {
        Some(&[]) => PASS,
        _ => fail("Server did not respond with empty aead", response),
    }
}

pub fn ignore_unknown_extra_aead(ke: &mut NtsKeConnection) -> TestResult {
    let response = ke.exchange(Request {
        aead: vec![0xFFFF, 15],
        ..Request::default()
    })?;

    match response.aead.as_deref() {
        Some(&[15]) => PASS,
        _ => fail("Server did not respond with empty aead", response),
    }
}

pub fn empty_message_resolves_in_error(ke: &mut NtsKeConnection) -> TestResult {
    let response = ke.exchange([NtsRecord::EndOfMessage])?;

    if response.errors != [1] {
        return fail(
            "Server did not respond with error to empty message",
            response,
        );
    }

    PASS
}

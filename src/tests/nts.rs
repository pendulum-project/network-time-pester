//! Tests that exercise the NTS Extension Fields for NTPv4
//!
//! These extensions are described in [RFC8915 section 5](https://datatracker.ietf.org/doc/html/rfc8915#name-nts-extension-fields-for-nt).

use crate::macros::{pester_assert, pester_assert_eq, pester_assert_response};
use crate::nts::NtsCookie;
use crate::udp::UdpConnection;
use crate::util::result::PASS;
use crate::TestResult;
use ntp_proto::{NtpPacket, NtsKeys, PollInterval};

/// Ensure the server correctly responds to a normal NTS request
pub fn happy(conn: &mut UdpConnection, cookie: NtsCookie, keys: &NtsKeys) -> TestResult {
    let (request, id) = NtpPacket::nts_poll_message(&cookie, 4, PollInterval::default());

    let response = pester_assert_response!(conn.pester_nts(request, keys)?);

    pester_assert!(
        response,
        response.valid_server_response(id, true),
        "Response did not match request"
    );

    pester_assert_eq!(
        response,
        response.new_cookies().count(),
        4,
        "Server did not respond with the expected number of cookies",
    );

    PASS
}

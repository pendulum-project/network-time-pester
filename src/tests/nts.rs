use crate::nts::NtsCookie;
use crate::udp::UdpConnection;
use crate::util::result::{fail, PASS};
use crate::{TestError, TestResult};
use ntp_proto::{NtpPacket, NtsKeys, PollInterval};

pub fn happy(conn: &mut UdpConnection, cookie: NtsCookie, keys: &NtsKeys) -> TestResult {
    let (request, id) = NtpPacket::nts_poll_message(&cookie, 4, PollInterval::default());

    let response = conn
        .pester_nts(request, keys)?
        .ok_or_else(|| TestError::Fail("Server did not respond".to_owned(), None))?;

    if !response.valid_server_response(id, true) {
        return fail("Response did not match request", response);
    }

    if response.new_cookies().count() != 4 {
        return fail(
            "Server did not respond with the expected number of cookies",
            response,
        );
    }

    PASS
}

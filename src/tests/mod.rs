//! This module contains a collection of test cases
//!
//! Every test case is implemented as a function wrapped by one of [udp_test], [nts_test], or [ke_test]. This module is
//! made public when the documentation is generated so that normal rust docstrings can be used the test cases.

use crate::nts::nts_test;
use crate::nts_ke::ke_test;
use crate::udp::udp_test;
use crate::TestCase;
use std::panic::UnwindSafe;

pub mod basic;
pub mod extensions;
pub mod nts;
pub mod nts_ke;

/// Generate a list of all currently implemented test cases
pub fn all_tests() -> impl Iterator<Item = Box<dyn TestCase + UnwindSafe>> {
    [
        udp_test(basic::test_responds_to_version_4),
        udp_test(basic::test_ignores_version_5),
        udp_test(extensions::test_unknown_extensions_are_ignored),
        udp_test(extensions::test_unique_id_is_returned),
        nts_test(nts::happy),
        ke_test(nts_ke::happy),
        ke_test(nts_ke::error_on_unknown_next_protocol),
        ke_test(nts_ke::ignore_unknown_extra_protocols),
        ke_test(nts_ke::error_on_unknown_aead),
        ke_test(nts_ke::ignore_unknown_extra_aead),
        ke_test(nts_ke::empty_message_resolves_in_error),
    ]
    .into_iter()
}

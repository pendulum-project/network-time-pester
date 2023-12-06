use crate::nts_ke::ke_test;
use crate::udp::udp_test;
use crate::TestCase;

pub mod basic;
pub mod extensions;
pub mod nts_ke;

/// Generate a list of all currently implemented test cases
pub fn all_tests() -> impl Iterator<Item = Box<dyn TestCase>> {
    [
        udp_test(basic::test_responds_to_version_4),
        udp_test(basic::test_ignores_version_5),
        udp_test(extensions::test_unknown_extensions_are_ignored),
        udp_test(extensions::test_unique_id_is_returned),
        ke_test(nts_ke::happy),
        ke_test(nts_ke::error_on_unknown_next_protocol),
        ke_test(nts_ke::ignore_unknown_extra_protocols),
        ke_test(nts_ke::error_on_unknown_aead),
        ke_test(nts_ke::ignore_unknown_extra_aead),
        ke_test(nts_ke::empty_message_resolves_in_error),
    ]
    .into_iter()
}

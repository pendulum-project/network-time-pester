use crate::TestCase;

pub mod basic;
pub mod extensions;

/// Generate a list of all currently implemented test cases
pub fn all_tests() -> Vec<Box<dyn TestCase>> {
    vec![
        Box::new(basic::test_responds_to_version_4),
        Box::new(basic::test_ignores_version_5),
        Box::new(extensions::test_unknown_extensions_are_ignored),
        Box::new(extensions::test_unique_id_is_returned),
    ]
}

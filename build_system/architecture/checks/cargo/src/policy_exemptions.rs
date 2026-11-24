pub const CARGO_RULE_OWNER: &str = "build_system/architecture/checks/cargo/src/cargo_rules.rs";
pub const CARGO_TEST_OWNER: &str =
    "build_system/architecture/checks/cargo/unit_tests/cargo_policy_tests.rs";
const CARGO_RULE_DECLARATION: &str = "const CARGO_TEST_BINARY_ENVIRONMENT:";
const ESCAPED_FIXTURE_QUOTE: &str = "\\\"";

pub fn owns_cargo_environment_fixture_token(relative: &str, line: &str) -> bool {
    (relative == CARGO_RULE_OWNER && line.trim_start().starts_with(CARGO_RULE_DECLARATION))
        || (relative == CARGO_TEST_OWNER && line.contains(ESCAPED_FIXTURE_QUOTE))
}

use crate::policy_exemptions::{
    CARGO_RULE_OWNER, CARGO_TEST_OWNER, owns_cargo_environment_fixture_token,
};

const CARGO_RULE_DECLARATION: &str = "const CARGO_TEST_BINARY_ENVIRONMENT:";
const ESCAPED_FIXTURE_QUOTE: &str = "\\\"";
const UNRELATED_SOURCE: &str = "core/src/session.rs";
const UNRELATED_DECLARATION: &str = "const UNRELATED: &str = value;";

#[test]
fn exemption_requires_the_exact_owner_and_fixture_shape() {
    assert!(owns_cargo_environment_fixture_token(
        CARGO_RULE_OWNER,
        CARGO_RULE_DECLARATION
    ));
    assert!(owns_cargo_environment_fixture_token(
        CARGO_TEST_OWNER,
        ESCAPED_FIXTURE_QUOTE
    ));
    assert!(!owns_cargo_environment_fixture_token(
        CARGO_RULE_OWNER,
        UNRELATED_DECLARATION
    ));
    assert!(!owns_cargo_environment_fixture_token(
        UNRELATED_SOURCE,
        ESCAPED_FIXTURE_QUOTE
    ));
}

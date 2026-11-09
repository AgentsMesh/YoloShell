use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};
use yolo_architecture_policy_contracts::process_signal as contract;

const FUTURE_PATH: &str = "core/crates/runtime/engine/process/src/future_retained_pattern_clean.rs";

#[test]
fn pattern_modifiers_do_not_become_retained_bindings() {
    assert_allowed(
        r#"struct Other;
fn allowed(authority: RetainedExactProcess, mut other: Other) {
    let mut target = authority;
    let clean = &mut other;
    clean.enforce_containment();
    consume(target);
}"#,
    );
}

#[test]
fn match_guard_identifiers_do_not_become_arm_bindings() {
    assert_allowed(
        r#"struct Other;
fn allowed(source: Option<RetainedExactProcess>, mut other: Other) {
    match source {
        Some(target) if other.ready() => other.enforce_containment(),
        None => consume(other),
    }
}"#,
    );
}

fn assert_allowed(source: &str) {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(FUTURE_PATH, source);
    let result = messages(&fixture);
    assert!(!has_policy(&result), "{result:#?}");
}

fn has_policy(messages: &[String]) -> bool {
    messages
        .iter()
        .any(|message| message.contains(contract::AUTHORITY_CONSTRUCTOR_MESSAGE))
}

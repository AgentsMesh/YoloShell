use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};
use yolo_architecture_policy_contracts::process_signal as contract;

const FUTURE_PATH: &str = "core/crates/runtime/engine/process/src/future_retained_closures.rs";

#[test]
fn immediately_invoked_closures_propagate_arguments() {
    assert_rejected(
        r#"fn bypass(authority: RetainedExactProcess) {
    (|mut target| target.enforce_containment())(authority);
}"#,
    );
}

#[test]
fn stored_closures_propagate_later_call_arguments() {
    assert_rejected(
        r#"fn bypass(authority: RetainedExactProcess) {
    let callback = |mut target| target.begin_containment();
    callback(authority);
}"#,
    );
}

#[test]
fn typed_closure_parameters_are_retained() {
    assert_rejected(
        r#"fn bypass(value: Unknown) {
    let callback = |mut target: RetainedExactProcess| target.enforce_containment();
    callback(value);
}"#,
    );
}

#[test]
fn nested_destructured_closure_parameters_propagate() {
    assert_rejected(
        r#"fn bypass(authority: RetainedExactProcess) {
    (|(Ok(mut target) | Err(mut target), _)| target.begin_containment())(
        (Ok::<_, RetainedExactProcess>(authority), ()),
    );
}"#,
    );
}

#[test]
fn captured_authority_in_the_body_does_not_taint_an_unrelated_parameter() {
    assert_allowed(
        r#"struct Other;
fn allowed(authority: RetainedExactProcess, other: Other) {
    let callback = |mut target: Other| {
        consume(&authority);
        target.enforce_containment();
    };
    callback(other);
}"#,
    );
}

#[test]
fn positional_arguments_taint_only_the_corresponding_parameter() {
    for source in [
        r#"struct Other;
fn allowed(authority: RetainedExactProcess, other: Other) {
    (|retained, mut clean| {
        consume(retained);
        clean.enforce_containment();
    })(authority, other);
}"#,
        r#"struct Other;
fn allowed(authority: RetainedExactProcess, other: Other) {
    let callback = |retained, mut clean| {
        consume(retained);
        clean.enforce_containment();
    };
    callback(authority, other);
}"#,
    ] {
        assert_allowed(source);
    }
}

fn assert_rejected(source: &str) {
    let result = inspect(source);
    assert!(has_policy(&result), "{result:#?}");
}

fn assert_allowed(source: &str) {
    let result = inspect(source);
    assert!(!has_policy(&result), "{result:#?}");
}

fn inspect(source: &str) -> Vec<String> {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(FUTURE_PATH, source);
    messages(&fixture)
}

fn has_policy(messages: &[String]) -> bool {
    messages
        .iter()
        .any(|message| message.contains(contract::AUTHORITY_CONSTRUCTOR_MESSAGE))
}

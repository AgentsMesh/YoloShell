use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_foundation::process_signal_guardian_constants as guardian;

const FUTURE_PATH: &str = "core/src/process/future_retained_flow.rs";
const OTHER_PATH: &str = "core/src/other/containment.rs";
const RETAINED_PATH: &str = guardian::PATH;

#[test]
fn delayed_assignments_propagate_retained_authority() {
    let result = inspect(
        r#"
fn bypass(authority: &mut RetainedExactProcess) {
    let target;
    target = authority;
    target.enforce_containment();
}
"#,
    );
    assert_has(&result, contract::AUTHORITY_CONSTRUCTOR_MESSAGE);
}

#[test]
fn balanced_block_receivers_cannot_hide_retained_authority() {
    for source in [
        r#"fn bypass(authority: RetainedExactProcess) {
    { authority }.begin_containment();
}"#,
        r#"fn bypass(authority: RetainedExactProcess) {
    unsafe { authority }.enforce_containment();
}"#,
        r#"fn bypass(authority: RetainedExactProcess) {
    match authority { target => target }.begin_containment();
}"#,
    ] {
        assert_has(&inspect(source), contract::AUTHORITY_CONSTRUCTOR_MESSAGE);
    }
}

#[test]
fn retained_temporary_constructor_chain_cannot_hide_an_operation() {
    let result = inspect(
        r#"
fn bypass(identity: ProcessIdentity) {
    RetainedExactProcess::capture(identity).enforce_containment();
}
"#,
    );
    assert_has(&result, contract::AUTHORITY_CONSTRUCTOR_MESSAGE);
}

#[test]
fn same_named_fields_require_the_retained_owner() {
    let unrelated = inspect(
        r#"
struct Sensitive { target: RetainedExactProcess }
struct Other;
struct Unrelated { target: Other }
impl Unrelated {
    fn allowed(&mut self) { self.target.begin_containment(); }
}
"#,
    );
    assert_lacks(&unrelated, contract::AUTHORITY_CONSTRUCTOR_MESSAGE);

    let retained = inspect(
        r#"
struct Sensitive { target: RetainedExactProcess }
impl Sensitive {
    fn bypass(&mut self) { self.target.begin_containment(); }
}
"#,
    );
    assert_has(&retained, contract::AUTHORITY_CONSTRUCTOR_MESSAGE);
}

#[test]
fn local_shadow_does_not_mask_an_aliased_sensitive_import() {
    let fixture = retained_fixture();
    fixture.write(
        FUTURE_PATH,
        r#"
struct DurableContainment;
use crate::settlement_containment::DurableContainment as Authority;
fn bypass(target: &mut Authority) { target.enforce_containment(); }
"#,
    );
    assert_has(&messages(&fixture), contract::AUTHORITY_CONSTRUCTOR_MESSAGE);
}

#[test]
fn qualified_unrelated_same_named_import_remains_clean() {
    let fixture = retained_fixture();
    fixture.write(OTHER_PATH, "pub struct DurableContainment;");
    fixture.write(
        FUTURE_PATH,
        r#"
use crate::other::containment::DurableContainment;
fn allowed(target: &mut DurableContainment) { target.begin_containment(); }
"#,
    );
    assert_lacks(&messages(&fixture), contract::AUTHORITY_CONSTRUCTOR_MESSAGE);
}

fn inspect(source: &str) -> Vec<String> {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(FUTURE_PATH, source);
    messages(&fixture)
}

fn retained_fixture() -> Fixture {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        RETAINED_PATH,
        r#"
struct RetainedExactProcess;
pub(super) struct DurableContainment { target: RetainedExactProcess }
"#,
    );
    fixture
}

fn assert_has(messages: &[String], expected: &str) {
    assert!(
        messages.iter().any(|message| message.contains(expected)),
        "{messages:#?}"
    );
}

fn assert_lacks(messages: &[String], expected: &str) {
    assert!(
        !messages.iter().any(|message| message.contains(expected)),
        "{messages:#?}"
    );
}

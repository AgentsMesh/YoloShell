use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_foundation::process_signal_guardian_constants as guardian;

const FUTURE_PATH: &str = "core/crates/runtime/engine/custody/guardian/src/future_containment.rs";
const MOVED_OWNER: &str = "MovedExactTarget";

#[test]
fn exact_target_field_is_bound_to_the_existing_guardian_owner() {
    assert_fixture(guardian::PATH, guardian::TARGET_OWNER, 0);
    assert_fixture(guardian::PATH, MOVED_OWNER, 1);
    assert_fixture(FUTURE_PATH, guardian::TARGET_OWNER, 1);
}

#[test]
fn test_sources_cannot_expand_production_authority_ownership() {
    assert_fixture(
        "core/crates/runtime/engine/worker/integration_tests/future_tests.rs",
        guardian::TARGET_OWNER,
        0,
    );
}

fn assert_fixture(path: &str, owner: &str, types: usize) {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(path, &type_source(owner));
    let result = messages(&fixture);
    assert_eq!(
        matches(&result, contract::AUTHORITY_TYPE_MESSAGE),
        types,
        "{result:#?}"
    );
}

fn type_source(owner: &str) -> String {
    format!("pub(super) struct {owner} {{ authority: RetainedExactProcess }}\n")
}

fn matches(messages: &[String], expected: &str) -> usize {
    messages
        .iter()
        .filter(|message| message.contains(expected))
        .count()
}

use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_foundation::process_signal_guardian_constants as guardian;

const CONSTRUCTOR_POLICY: &str = "authority constructor moved";
const FUTURE_PATH: &str = "core/crates/runtime/engine/process/src/future_retained_exact.rs";
const MOVED_TARGET_OWNER: &str = "MovedExactTarget";
const TYPE_POLICY: &str = "authority type escaped";

#[test]
fn exact_observer_fixture_has_one_constructor_authority_site() {
    let allowed = Fixture::new();
    write_contract(&allowed);
    allowed.write(
        contract::RETAINED_EXACT_DOUBLE_FORK_TESTS_PATH,
        &fixture_function("discovery_and_no_proof_custody_keep_refresh_semantics"),
    );
    let result = messages(&allowed);
    assert!(!contains(&result, CONSTRUCTOR_POLICY));
    assert!(!contains(&result, TYPE_POLICY));

    let wrong_owner = Fixture::new();
    write_contract(&wrong_owner);
    wrong_owner.write(
        contract::RETAINED_EXACT_DOUBLE_FORK_TESTS_PATH,
        &fixture_function("moved_fixture"),
    );
    assert!(contains(&messages(&wrong_owner), CONSTRUCTOR_POLICY));

    let wrong_path = Fixture::new();
    write_contract(&wrong_path);
    wrong_path.write(
        FUTURE_PATH,
        &fixture_function("discovery_and_no_proof_custody_keep_refresh_semantics"),
    );
    let result = messages(&wrong_path);
    assert!(contains(&result, CONSTRUCTOR_POLICY));
    assert!(contains(&result, TYPE_POLICY));
}

#[test]
fn guardian_capture_is_bound_to_its_exact_site() {
    assert_guardian_operation(contract::CAPTURE, contract::CAPTURE);
}

#[test]
fn guardian_graceful_containment_is_bound_to_its_exact_site() {
    assert_guardian_operation(contract::BEGIN_CONTAINMENT, guardian::GRACEFUL_FUNCTION);
    assert_guardian_method_call(contract::BEGIN_CONTAINMENT, guardian::GRACEFUL_FUNCTION);
}

#[test]
fn guardian_force_containment_is_bound_to_its_exact_site() {
    assert_guardian_operation(contract::ENFORCE_CONTAINMENT, guardian::FORCE_FUNCTION);
    assert_guardian_method_call(contract::ENFORCE_CONTAINMENT, guardian::FORCE_FUNCTION);
}

#[test]
fn guardian_exact_observation_is_bound_to_its_exact_site() {
    assert_guardian_method_call(guardian::OBSERVE_OPERATION, guardian::OBSERVATION_FUNCTION);
}

#[test]
fn retained_method_receiver_aliases_cannot_bypass_guardian_policy() {
    let parameter = format!(
        "impl {MOVED_TARGET_OWNER} {{ fn {}(target: &mut RetainedExactProcess) {{ \
         target.{}(); }} }}\n",
        guardian::GRACEFUL_FUNCTION,
        contract::BEGIN_CONTAINMENT,
    );
    assert_fixture(guardian::PATH, &parameter, 1, 0);

    let field = format!(
        "struct {MOVED_TARGET_OWNER} {{ target: RetainedExactProcess }} \
         impl {MOVED_TARGET_OWNER} {{ fn {}(&mut self) {{ \
         self.target.{}(); }} }}\n",
        guardian::GRACEFUL_FUNCTION,
        contract::BEGIN_CONTAINMENT,
    );
    assert_fixture(guardian::PATH, &field, 1, 0);

    let rebound = format!(
        "impl {MOVED_TARGET_OWNER} {{ fn {}(authority: &mut RetainedExactProcess) {{ \
         let target = authority; \
         target.{}(); }} }}\n",
        guardian::GRACEFUL_FUNCTION,
        contract::BEGIN_CONTAINMENT,
    );
    assert_fixture(guardian::PATH, &rebound, 1, 0);
}

#[test]
fn retained_type_aliases_and_fields_cannot_hide_operations() {
    let alias = format!(
        "use crate::process::RetainedExactProcess::{{self as R}};\n\
         type Nested = R;\nfn bypass(target: &mut Nested) {{ target.{}(); }}\n",
        contract::BEGIN_CONTAINMENT,
    );
    assert_fixture(FUTURE_PATH, &alias, 2, 1);

    let field = format!(
        "struct Wrapper {{ target: RetainedExactProcess }}\n\
         impl Wrapper {{ fn bypass(&mut self) {{ self.target.{}(); }} }}\n",
        contract::ENFORCE_CONTAINMENT,
    );
    assert_fixture(FUTURE_PATH, &field, 1, 1);
}

#[test]
fn unrelated_receivers_with_shared_method_names_are_ignored() {
    let source = "struct Other;\nfn bypass(target: &mut Other) { \
                  target.begin_containment(); target.observe(); }\n";
    assert_fixture(FUTURE_PATH, source, 0, 0);
}

fn fixture_function(name: &str) -> String {
    format!("fn {name}() {{ RetainedExactProcess::capture(identity); }}\n")
}

fn assert_guardian_operation(operation: &str, function: &str) {
    let allowed = guardian_operation(guardian::TARGET_OWNER, function, operation);
    assert_fixture(guardian::PATH, &allowed, 0, 0);

    let wrong_owner = guardian_operation(MOVED_TARGET_OWNER, function, operation);
    assert_fixture(guardian::PATH, &wrong_owner, 1, 0);

    let wrong_function = guardian_operation(guardian::TARGET_OWNER, contract::SETTLE, operation);
    assert_fixture(guardian::PATH, &wrong_function, 1, 0);

    assert_fixture(FUTURE_PATH, &allowed, 1, 1);
}

fn guardian_operation(owner: &str, function: &str, operation: &str) -> String {
    format!(
        "impl {owner} {{ fn {function}() {{ \
         RetainedExactProcess::{operation}(authority); }} }}\n"
    )
}

fn assert_guardian_method_call(operation: &str, function: &str) {
    let allowed = guardian_method_call(guardian::TARGET_OWNER, function, operation);
    assert_fixture(guardian::PATH, &allowed, 0, 0);

    let wrong_owner = guardian_method_call(MOVED_TARGET_OWNER, function, operation);
    assert_fixture(guardian::PATH, &wrong_owner, 1, 0);

    let wrong_function = guardian_method_call(guardian::TARGET_OWNER, contract::SETTLE, operation);
    assert_fixture(guardian::PATH, &wrong_function, 1, 0);

    let outside = guardian_method_call_with_receiver(
        guardian::TARGET_OWNER,
        function,
        operation,
        "authority",
    );
    assert_fixture(FUTURE_PATH, &outside, 1, 1);
}

fn guardian_method_call(owner: &str, function: &str, operation: &str) -> String {
    guardian_method_call_with_receiver(owner, function, operation, "target")
}

fn guardian_method_call_with_receiver(
    owner: &str,
    function: &str,
    operation: &str,
    receiver: &str,
) -> String {
    format!(
        "impl {owner} {{ fn {function}({receiver}: &mut RetainedExactProcess) {{ \
         {receiver}.{operation}(); }} }}\n"
    )
}

fn assert_fixture(path: &str, source: &str, constructors: usize, types: usize) {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(path, source);
    let result = messages(&fixture);
    assert_eq!(
        matches(&result, CONSTRUCTOR_POLICY),
        constructors,
        "{result:#?}"
    );
    assert_eq!(matches(&result, TYPE_POLICY), types, "{result:#?}");
}

fn contains(messages: &[String], expected: &str) -> bool {
    messages.iter().any(|message| message.contains(expected))
}

fn matches(messages: &[String], expected: &str) -> usize {
    messages
        .iter()
        .filter(|message| message.contains(expected))
        .count()
}

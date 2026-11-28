use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_policy_contracts::process_signal_retained as retained;
use yolo_architecture_process_signal_retained_policy::process_signal_retained_api::EXACT_TEST_SUPPORT_PATH;

const FUTURE_PATH: &str = "core/crates/runtime/engine/process/src/future_retained_surface.rs";
const UNICODE_IDENTIFIER: &str = "\u{6743}\u{9650}";

#[test]
fn unicode_identifiers_fail_closed_without_rejecting_comments_or_strings() {
    let binding = format!(
        "fn bypass({UNICODE_IDENTIFIER}: &mut RetainedExactProcess) {{ \
         {UNICODE_IDENTIFIER}.begin_containment(); }}"
    );
    let rejected = inspect(FUTURE_PATH, &binding);
    assert_has(&rejected, retained::UNSUPPORTED_IDENTIFIER_MESSAGE);

    let inert =
        format!("const MESSAGE: &str = \"{UNICODE_IDENTIFIER}\";\n// {UNICODE_IDENTIFIER}\n");
    let accepted = inspect(FUTURE_PATH, &inert);
    assert_lacks(&accepted, retained::UNSUPPORTED_IDENTIFIER_MESSAGE);
}

#[test]
fn macro_generated_public_retained_surfaces_fail_closed() {
    for source in [
        r#"
macro_rules! declare_wrapper { ($target:ty) => { pub struct Wrapper($target); } }
declare_wrapper!(RetainedExactProcess);
"#,
        r#"
macro_rules! declare_alias { ($target:ty) => { pub type Alias = $target; } }
declare_alias!(RetainedExactProcess);
"#,
    ] {
        assert_has(
            &inspect(FUTURE_PATH, source),
            contract::AUTHORITY_TYPE_MESSAGE,
        );
    }
}

#[test]
fn module_level_closure_operations_are_rejected() {
    let result = inspect(
        FUTURE_PATH,
        r#"
static AUTHORITY: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    RetainedExactProcess::capture(identity);
});
"#,
    );
    assert_has(&result, contract::AUTHORITY_CONSTRUCTOR_MESSAGE);
}

#[test]
fn unknown_retained_exact_process_methods_are_rejected() {
    let result = inspect(
        contract::EXACT_TERMINATION_PATH,
        r#"
struct RetainedExactProcess;
impl RetainedExactProcess {
    fn unclassified(&self) {}
}

"#,
    );
    assert_has(&result, retained::UNCLASSIFIED_METHOD_MESSAGE);
}

#[test]
fn exact_public_retained_process_methods_are_the_only_classified_surface() {
    let public = r#"
struct ProcessIdentity;
struct RetainedExactProcess;
impl RetainedExactProcess {
    pub fn capture(identity: ProcessIdentity) -> Result<Option<Self>> { todo!() }
    pub fn is_running(&mut self) -> Result<bool> { todo!() }
    pub fn begin_containment(&mut self) -> Result<bool> { todo!() }
    pub fn enforce_containment(&mut self) -> Result<bool> { todo!() }
}
"#;
    assert_lacks(
        &inspect(contract::EXACT_TERMINATION_PATH, public),
        retained::UNCLASSIFIED_METHOD_MESSAGE,
    );

    let crate_private = public.replacen("pub fn capture", "pub(crate) fn capture", 1);
    assert_has(
        &inspect(contract::EXACT_TERMINATION_PATH, &crate_private),
        retained::UNCLASSIFIED_METHOD_MESSAGE,
    );
}

#[test]
fn adversarial_helpers_require_the_exact_feature_gated_surface() {
    let accepted = r#"
struct RetainedExactProcess;
impl RetainedExactProcess {
    #[cfg(feature = "adversarial-test-hooks")]
    pub fn capture_pty_for_test(identity: ProcessIdentity, proof: super::PtySessionProof,) -> Result<Option<Self>> { todo!() }
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    fn tree_mut(&mut self) -> &mut CapturedProcessTree { todo!() }
}
"#;
    assert_lacks(
        &inspect(contract::EXACT_TERMINATION_PATH, accepted),
        retained::UNCLASSIFIED_METHOD_MESSAGE,
    );
    for changed in [
        accepted.replacen("adversarial-test-hooks", "unrestricted", 1),
        accepted.replacen("pub fn capture_pty_for_test", "fn capture_pty_for_test", 1),
    ] {
        assert_has(
            &inspect(contract::EXACT_TERMINATION_PATH, &changed),
            retained::UNCLASSIFIED_METHOD_MESSAGE,
        );
    }
}

#[test]
fn component_retained_helpers_are_classified_only_at_their_owned_path() {
    let source = r#"
struct RetainedExactProcess;
impl RetainedExactProcess {
    pub fn fail_next_refresh_for_test(&mut self, detail: &str) { todo!() }
    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn is_exact_observer_for_test(&self) -> bool { todo!() }
    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn state_snapshot_for_test(&self) -> crate::CapturedProcessTreeStateSnapshot { todo!() }
}
"#;
    assert_lacks(
        &inspect_api(EXACT_TEST_SUPPORT_PATH, source),
        retained::UNCLASSIFIED_METHOD_MESSAGE,
    );
    assert_has(
        &inspect_api(FUTURE_PATH, source),
        retained::UNCLASSIFIED_METHOD_MESSAGE,
    );
    for mutation in [
        source.replace("runtime-component-tests", "ordinary-feature"),
        source.replace("-> bool", "-> RetainedExactProcess"),
        source.replace(
            retained::IS_EXACT_OBSERVER_FOR_TEST_FUNCTION,
            "unclassified_helper",
        ),
    ] {
        assert_has(
            &inspect_api(EXACT_TEST_SUPPORT_PATH, &mutation),
            retained::UNCLASSIFIED_METHOD_MESSAGE,
        );
    }
}

fn inspect_api(path: &str, source: &str) -> Vec<String> {
    use yolo_architecture_process_signal_retained_policy::process_signal_retained_api;
    let file = yolo_architecture_foundation::SourceFile {
        absolute: path.into(),
        relative: path.into(),
        text: source.into(),
    };
    let parsed = yolo_architecture_syntax::rust_source::RustSource::parse(source);
    let mut violations = Vec::new();
    process_signal_retained_api::inspect(&file, &parsed, &mut violations);
    violations
        .into_iter()
        .map(|violation| violation.message)
        .collect()
}

fn inspect(path: &str, source: &str) -> Vec<String> {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(path, source);
    messages(&fixture)
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

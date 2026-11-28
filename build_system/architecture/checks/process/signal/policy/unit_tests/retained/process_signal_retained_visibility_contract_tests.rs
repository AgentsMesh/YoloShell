use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_policy_contracts::process_signal_retained as retained;
use yolo_architecture_process_signal_foundation::process_signal_guardian_constants as guardian;

const ADAPTER_TYPES: &str = "struct RetainedExactProcess;\n\
                             struct DurableContainment { target: RetainedExactProcess }\n\
                             struct DurableHandshake;";

#[test]
fn retired_launcher_return_surfaces_are_rejected() {
    let accepted = r#"
pub(super) fn capture_launcher(
    identity: Option<crate::process::ProcessIdentity>,
) -> Result<Option<crate::process::RetainedExactProcess>> { panic!() }
"#;
    assert_type_violation(guardian::PATH, accepted);
    assert_type_violation(
        guardian::PATH,
        &accepted.replace("pub(super)", "pub(crate)"),
    );
    assert_type_violation(
        guardian::PATH,
        &accepted.replace("capture_launcher", "capture_other"),
    );
    assert_type_violation(
        guardian::PATH,
        &accepted.replace("ProcessIdentity>,", "ProcessIdentity>"),
    );
}

#[test]
fn retired_launcher_parameter_surfaces_are_rejected() {
    let accepted = r#"
pub(super) fn settle_failure(
    error: anyhow::Error,
    child: crate::process::DetachedChildCustody,
    launcher: Option<crate::process::RetainedExactProcess>,
    watchdog: Option<crate::process::ProcessIdentity>,
) -> anyhow::Error { panic!() }
"#;
    assert_type_violation(guardian::PATH, accepted);
    assert_type_violation(
        guardian::PATH,
        &accepted.replace("pub(super)", "pub(crate)"),
    );
    assert_type_violation(
        guardian::PATH,
        &accepted.replace("settle_failure", "settle_other"),
    );
    assert_type_violation(
        guardian::PATH,
        &accepted.replace("launcher:", "retained_launcher:"),
    );
}

#[test]
fn retained_method_allowlist_requires_visibility_cfg_and_signature() {
    let accepted = r#"
struct RetainedExactProcess;
impl RetainedExactProcess {
    fn destructive(tree: CapturedProcessTree) -> Self { panic!() }
}
"#;
    assert_lacks(
        &inspect(contract::EXACT_TERMINATION_PATH, accepted),
        retained::UNCLASSIFIED_METHOD_MESSAGE,
    );
    for changed in [
        accepted.replace("fn destructive", "pub(crate) fn destructive"),
        accepted.replace("destructive(tree", "destructive(other"),
        accepted.replace("fn destructive", "#[cfg(test)] fn destructive"),
    ] {
        assert_has(
            &inspect(contract::EXACT_TERMINATION_PATH, &changed),
            retained::UNCLASSIFIED_METHOD_MESSAGE,
        );
    }
}

#[test]
fn retired_watchdog_associated_type_surfaces_are_rejected() {
    let accepted = adapter_source(ADAPTER_TYPES);
    assert_type_violation(guardian::PATH, &accepted);
    for changed in [
        accepted.replace("impl Handshake", "#[cfg(test)] impl Handshake"),
        accepted.replace("Handshake for", "OtherHandshake for"),
        accepted.replace("for DurableHandshake", "for OtherHandshake"),
        accepted.replace("type Containment =", "type Authority ="),
    ] {
        assert_type_violation(guardian::PATH, &changed);
    }
}

fn adapter_source(prefix: &str) -> String {
    format!(
        "{prefix}\ntrait Handshake {{ type Containment; }}\n\
         impl Handshake for DurableHandshake {{\n\
             type Containment = DurableContainment;\n\
         }}"
    )
}

fn inspect(path: &str, source: &str) -> Vec<String> {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(path, source);
    messages(&fixture)
}

fn assert_type_violation(path: &str, source: &str) {
    assert_has(&inspect(path, source), contract::AUTHORITY_TYPE_MESSAGE);
}

#[rustfmt::skip]
fn assert_has(messages: &[String], expected: &str) {
    assert!(messages.iter().any(|message| message.contains(expected)), "{messages:#?}");
}

#[rustfmt::skip]
fn assert_lacks(messages: &[String], expected: &str) {
    assert!(!messages.iter().any(|message| message.contains(expected)), "{messages:#?}");
}

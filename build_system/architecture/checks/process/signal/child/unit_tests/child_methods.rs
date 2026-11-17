use super::constants::{SAMPLE_PATH, TEST_SAMPLE_PATH};
use super::support::inspect;
use crate::process_signal_child_methods;
use yolo_architecture_policy_contracts::process_signal as contract;

#[test]
fn explicit_false_disables_implicit_kill_for_method_and_ufcs_forms() {
    for arguments in [
        "false",
        "false,",
        "&mut command, false",
        "&mut command, false,",
    ] {
        let source = format!(
            "fn operation() {{ command.{}({arguments}); }}",
            contract::KILL_ON_DROP_METHOD
        );
        assert!(inspect(SAMPLE_PATH, &source, process_signal_child_methods::inspect).is_empty());
    }
}

#[test]
fn missing_or_computed_kill_on_drop_values_cannot_enable_implicit_termination() {
    for arguments in ["", "true", "setting", "false || setting", "false, true"] {
        let source = format!(
            "fn operation() {{ command.{}({arguments}); }}",
            contract::KILL_ON_DROP_METHOD
        );
        assert_eq!(
            inspect(SAMPLE_PATH, &source, process_signal_child_methods::inspect).len(),
            1
        );
    }
    assert_eq!(
        inspect(
            SAMPLE_PATH,
            contract::KILL_ON_DROP_METHOD,
            process_signal_child_methods::inspect
        )
        .len(),
        1
    );
}

#[test]
fn implicit_lifecycle_operations_have_only_explicit_test_exemptions() {
    for method in contract::IMPLICIT_CHILD_TERMINATION_NAMES {
        let body = format!("fn operation() {{ command.{method}(true); }}");
        assert_eq!(
            inspect(SAMPLE_PATH, &body, process_signal_child_methods::inspect).len(),
            1
        );
        assert!(
            inspect(
                TEST_SAMPLE_PATH,
                &body,
                process_signal_child_methods::inspect
            )
            .is_empty()
        );
        let guarded = format!("#[cfg(all(test, unix))] {body}");
        assert!(inspect(SAMPLE_PATH, &guarded, process_signal_child_methods::inspect).is_empty());
        let unguarded = format!("#[cfg(any(test, unix))] {body}");
        assert_eq!(
            inspect(
                SAMPLE_PATH,
                &unguarded,
                process_signal_child_methods::inspect
            )
            .len(),
            1
        );
    }
}

#[test]
fn seccomp_owner_allows_only_its_two_prctl_options() {
    let allowed = "fn install_aarch64() { unsafe { libc::prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0); } }";
    assert!(
        inspect(
            contract::SECCOMP_OWNER_PATH,
            allowed,
            process_signal_child_methods::inspect
        )
        .is_empty()
    );
    let refused = "fn install_aarch64() { libc::prctl(PR_SET_PDEATHSIG, 1); }";
    assert!(
        !inspect(
            contract::SECCOMP_OWNER_PATH,
            refused,
            process_signal_child_methods::inspect
        )
        .is_empty()
    );
    assert_eq!(
        inspect(SAMPLE_PATH, allowed, process_signal_child_methods::inspect).len(),
        1
    );
    let wrong_function = "fn other() { libc::prctl(PR_SET_NO_NEW_PRIVS, 1); }";
    assert_eq!(
        inspect(
            contract::SECCOMP_OWNER_PATH,
            wrong_function,
            process_signal_child_methods::inspect
        )
        .len(),
        1
    );
}

use super::constants::{SAMPLE_PATH, TEST_SAMPLE_PATH};
use super::support::inspect;
use crate::process_signal_child_kills;
use yolo_architecture_policy_contracts::process_signal as contract;

#[test]
fn child_kill_namespace_imports_cannot_be_converted_to_values_or_aliases() {
    for (source, violations) in [
        (
            "use crate::typed::kill; fn operation() { kill::send(); kill::inspect(); }",
            0,
        ),
        (
            "use crate::typed::kill; fn operation() { kill::send(); let callback = kill; }",
            1,
        ),
        ("use crate::typed::kill;", 1),
        (
            "use crate::typed::kill as alias; fn operation() { alias::send(); }",
            1,
        ),
        ("use libc::{kill, raise};", 0),
        ("use crate::kill::{Type};", 0),
        ("use crate::kill::{kill as alias};", 1),
        ("use crate::kill::{kill}; fn operation() { kill(); }", 1),
        ("use libc::kill;", 0),
        ("fn operation() { libc::kill(pid, signal); }", 0),
        ("fn operation() { Child::kill::operation(); }", 0),
        ("fn operation() { let callback = Child::kill; }", 1),
    ] {
        assert_eq!(
            inspect(SAMPLE_PATH, source, process_signal_child_kills::inspect).len(),
            violations,
            "{source}"
        );
    }
}

#[test]
fn exact_std_owner_rejects_alternate_root_paths_argument_shapes_and_truncation() {
    for call in [
        "::std::process::Child::kill(&mut self.child)",
        "std::process::Child::kill(&mut self.child)",
        "::other::process::Child::kill(&mut self.child)",
        "::std::other::Child::kill(&mut self.child)",
        "::std::process::Other::kill(&mut self.child)",
        "::std::process::Child::kill(self)",
        "::std::process::Child::kill(&self.child)",
        "::std::process::Child::kill(&mut child)",
        "::std::process::Child::kill(&mut self.other)",
        "::std::process::Child::kill(&mut self.child, other)",
    ] {
        let source = format!(
            "impl {} {{ fn {}(&mut self) {{ {call}; }} }}",
            contract::DETACHED_CHILD_CUSTODY,
            contract::REQUEST_DIRECT_TERMINATION
        );
        let valid = call == "::std::process::Child::kill(&mut self.child)";
        assert_eq!(
            inspect(
                contract::DETACHED_CHILD_CUSTODY_PATH,
                &source,
                process_signal_child_kills::inspect
            )
            .is_empty(),
            valid,
            "{call}"
        );
    }
    let source = format!(
        "impl {} {{ fn {}(&mut self) {{ ::std::process::Child::kill; }} }}",
        contract::DETACHED_CHILD_CUSTODY,
        contract::REQUEST_DIRECT_TERMINATION
    );
    assert_eq!(
        inspect(
            contract::DETACHED_CHILD_CUSTODY_PATH,
            &source,
            process_signal_child_kills::inspect
        )
        .len(),
        1
    );
}

#[test]
fn child_termination_exemptions_do_not_escape_test_paths_or_cfg_guards() {
    let body = "fn operation() { child.kill(); }";
    assert_eq!(
        inspect(SAMPLE_PATH, body, process_signal_child_kills::inspect).len(),
        1
    );
    assert!(inspect(TEST_SAMPLE_PATH, body, process_signal_child_kills::inspect).is_empty());
    assert!(
        inspect(
            contract::PTY_SESSION_LINUX_INTEGRATION_HOLDER_PATH,
            body,
            process_signal_child_kills::inspect
        )
        .is_empty()
    );
    assert!(
        inspect(
            SAMPLE_PATH,
            &format!("#[cfg(test)] {body}"),
            process_signal_child_kills::inspect
        )
        .is_empty()
    );
    assert_eq!(
        inspect(
            SAMPLE_PATH,
            &format!("#[cfg(any(test, unix))] {body}"),
            process_signal_child_kills::inspect
        )
        .len(),
        1
    );
}

#[test]
fn non_call_tokens_and_module_names_do_not_trigger_child_termination() {
    for source in [
        "kill",
        "kill(",
        "fn kill() {}",
        "fn operation() { kill(); }",
        "fn operation() { value.kill; }",
        "use crate::kill::definition;",
    ] {
        assert!(
            inspect(SAMPLE_PATH, source, process_signal_child_kills::inspect).is_empty(),
            "{source}"
        );
    }
}

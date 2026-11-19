use crate::pty_alias_admission_rules as contract;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_policy_contracts::constants::*;

fn accepted(source: String) -> bool {
    let file = SourceFile {
        absolute: contract::RUNTIME_ALIAS_CLOSURE_PATH.into(),
        relative: contract::RUNTIME_ALIAS_CLOSURE_PATH.to_owned(),
        text: source,
    };
    let mut violations = Vec::new();
    crate::pty_alias_admission_semantics::inspect(&file, &mut violations);
    violations.is_empty()
}

fn poll(header: &str, body: &str) -> String {
    format!(
        "impl WorkerState {{ fn {PTY_ALIAS_POLL_CLOSE}(&self) {{ {header} {{ {body} self.{PTY_ALIAS_ADMITTED_CLOSE}(); }} }} }}"
    )
}

#[test]
fn each_try_lock_header_component_is_required_for_alias_closure() {
    let valid = format!("if let Ok(permit) = self.gate.{PTY_ALIAS_TRY_LOCK_OPERATION}()");
    assert!(accepted(poll(&valid, "{ observe(); }")));
    for header in [
        "if true".to_owned(),
        valid.replace("if let", "while let"),
        valid.replace("if let", "if"),
        valid.replace("Ok(permit)", "Err(permit)"),
        valid.replace("Ok(permit)", "Ok(other)"),
        valid.replace("Ok(permit)", "Ok[permit]"),
        valid.replace(" = ", " == "),
        valid.replace(PTY_ALIAS_TRY_LOCK_OPERATION, "other"),
        valid.replace("()", "(other)"),
    ] {
        assert!(!accepted(poll(&header, "")), "{header}");
    }
    assert!(!accepted(poll(&valid, "{ drop(permit); }")));
    assert!(accepted(poll(&valid, "drop(other); observe(permit);")));
}

#[test]
fn only_private_worker_definitions_and_typed_task_parameters_are_admitted() {
    assert!(accepted(format!(
        "impl WorkerState {{ fn {PTY_ALIAS_ADMITTED_CLOSE}(&self) {{}} }}"
    )));
    for prefix in ["pub", "pub(crate)", "pub(super)"] {
        assert!(!accepted(format!(
            "impl WorkerState {{ {prefix} fn {PTY_ALIAS_ADMITTED_CLOSE}(&self) {{}} }}"
        )));
    }
    assert!(!accepted(format!("fn {PTY_ALIAS_ADMITTED_CLOSE}() {{}}")));
    assert!(!accepted(format!(
        "impl Other {{ fn {PTY_ALIAS_ADMITTED_CLOSE}(&self) {{}} }}"
    )));
    assert!(!accepted(format!(
        "use module::{PTY_ALIAS_ADMITTED_CLOSE};"
    )));
    for parameter in [
        format!("_: {PTY_ALIAS_TASK_PERMIT}"),
        "_: &Other".to_owned(),
        format!("_: &mut {PTY_ALIAS_TASK_PERMIT}"),
        "".to_owned(),
    ] {
        assert!(
            !accepted(format!(
                "impl WorkerState {{ fn {PTY_ALIAS_TASK_ADMITTED_CLOSE}(&self, {parameter}) {{ self.{PTY_ALIAS_ADMITTED_CLOSE}(); }} }}"
            )),
            "{parameter}"
        );
    }
    assert!(accepted(format!(
        "impl WorkerState {{ fn {PTY_ALIAS_TASK_ADMITTED_CLOSE}(&self, _: &{PTY_ALIAS_TASK_PERMIT}) {{ self.{PTY_ALIAS_ADMITTED_CLOSE}(); }} }}"
    )));
}

#[test]
fn bypass_identifiers_are_rejected_at_every_alias_closure_site() {
    for symbol in [
        PTY_ALIAS_SYSTEM_PERMIT,
        PTY_ALIAS_SYSTEM_AUTHORITY_FIELD,
        PTY_ALIAS_SYSTEM_AUTHORITY_QUERY,
        PTY_ALIAS_SYSTEM_ADMITTED_CLOSE,
        PTY_ALIAS_SYSTEM_AUTHORIZED_CLOSE,
    ] {
        assert!(
            !accepted(format!("fn unrelated() {{ consume({symbol}); }}")),
            "{symbol}"
        );
    }
    assert!(accepted(format!(
        "impl Drop for WorkerState {{ fn drop(&mut self) {{ self.{PTY_ALIAS_ADMITTED_CLOSE}(); }} }}"
    )));
    assert!(!accepted(format!(
        "impl Other {{ fn drop(&mut self) {{ self.{PTY_ALIAS_ADMITTED_CLOSE}(); }} }}"
    )));
}

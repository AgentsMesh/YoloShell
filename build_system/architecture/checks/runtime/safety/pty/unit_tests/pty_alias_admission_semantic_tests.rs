use crate::pty_alias_admission_rules as contract;
use yolo_architecture_test_support::Fixture;

const POLICY_PREFIX: &str = "PTY alias admission";
const TERMINATION_POLICY: &str = "consume the typed mutation permit";
const BYPASS_POLICY: &str = "gate-external alias closure authority";

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .filter(|message| message.contains(POLICY_PREFIX))
        .collect()
}

fn termination(body: &str) -> String {
    format!("async fn terminate_pty_processes<P: PtyTerminationPermit>(permit: P) {{ {body} }}\n")
}

fn closure_source(poll_body: &str, extra: &str) -> String {
    format!(
        "impl WorkerState {{
fn poll_runtime_pty_alias_closure(&self) {{
    if let Ok(permit) = self.gate.try_lock_owned() {{ {poll_body} drop(permit); }}
}}
fn close_runtime_pty_aliases_task_admitted(&self, _: &AliasClosureMutationPermit) {{ self.close_runtime_pty_aliases_admitted(); }}
fn close_runtime_pty_aliases_admitted(&self) {{}}
{extra}
}}
impl Drop for WorkerState {{ fn drop(&mut self) {{ self.close_runtime_pty_aliases_admitted(); }} }}\n"
    )
}

#[test]
fn accepts_submission_then_commit_permit_drop_and_async_wait() {
    let fixture = Fixture::new();
    fixture.write(
        contract::PTY_PROCESS_TERMINATION_PATH,
        termination(
            "if let Some(owner) = state.pty_owner_handle() {
let submitted = owner.submit_termination(force)?;
permit.commit_alias_release(state, prepared);
drop(permit);
hook.await;
return submitted.wait().await;
}
#[cfg(test)] {
permit.commit_alias_release(state, prepared);
drop(permit);
return fallback.await;
}",
        ),
    );
    assert!(messages(&fixture).is_empty());
}

#[test]
fn rejects_missing_early_or_untyped_permit_consumption() {
    let cases = [
        "let submitted = owner.submit_termination(force)?; hook.await; drop(permit);",
        "drop(permit); let submitted = owner.submit_termination(force)?; submitted.wait().await;",
        "let submitted = owner.submit_termination(force)?; drop(other); submitted.wait().await;",
        "let submitted = owner.submit_termination(force)?; if false { drop(permit); } submitted.wait().await;",
    ];
    for body in cases {
        let fixture = Fixture::new();
        fixture.write(contract::PTY_PROCESS_TERMINATION_PATH, termination(body));
        let actual = messages(&fixture);
        assert!(
            actual
                .iter()
                .any(|message| message.contains(TERMINATION_POLICY))
        );
    }
    let fixture = Fixture::new();
    fixture.write(
        contract::PTY_PROCESS_TERMINATION_PATH,
        "async fn terminate_pty_processes(permit: OtherPermit) { let submitted = owner.submit_termination(force)?; drop(permit); submitted.wait().await; }\n",
    );
    assert!(
        messages(&fixture)
            .iter()
            .any(|message| message.contains(TERMINATION_POLICY))
    );
}

#[test]
fn accepts_only_pollable_or_typed_admitted_closure_routes() {
    let fixture = Fixture::new();
    fixture.write(
        contract::RUNTIME_ALIAS_CLOSURE_PATH,
        closure_source("self.close_runtime_pty_aliases_admitted();", ""),
    );
    assert!(messages(&fixture).is_empty());
}

#[test]
fn rejects_controller_weak_admission_direct_close() {
    let fixture = Fixture::new();
    let bypass = "fn close_runtime_pty_aliases_controller_admitted(&self, _: &ControllerMutationAdmission) { self.close_runtime_pty_aliases_admitted(); }";
    fixture.write(
        contract::RUNTIME_ALIAS_CLOSURE_PATH,
        closure_source("self.close_runtime_pty_aliases_admitted();", bypass),
    );
    assert!(
        messages(&fixture)
            .iter()
            .any(|message| message.contains(BYPASS_POLICY))
    );
}

#[test]
fn rejects_system_authorized_and_arbitrary_direct_closure_bypasses() {
    let bypasses = [
        "fn close_runtime_pty_aliases_system_admitted(&self, _: &SystemMutationPermit) { self.close_runtime_pty_aliases_admitted(); }",
        "fn bypass(&self) { if self.system_release_is_authorized() { self.close_runtime_pty_aliases_admitted(); } }",
    ];
    for bypass in bypasses {
        let fixture = Fixture::new();
        fixture.write(
            contract::RUNTIME_ALIAS_CLOSURE_PATH,
            closure_source("self.close_runtime_pty_aliases_admitted();", bypass),
        );
        let actual = messages(&fixture);
        assert!(actual.iter().any(|message| message.contains(BYPASS_POLICY)));
    }
    let fixture = Fixture::new();
    let public = closure_source("self.close_runtime_pty_aliases_admitted();", "").replace(
        "fn close_runtime_pty_aliases_admitted",
        "pub(crate) fn close_runtime_pty_aliases_admitted",
    );
    fixture.write(contract::RUNTIME_ALIAS_CLOSURE_PATH, public);
    assert!(
        messages(&fixture)
            .iter()
            .any(|message| message.contains(BYPASS_POLICY))
    );
}

#[test]
fn rejects_direct_poll_closure_before_try_lock_admission() {
    let fixture = Fixture::new();
    let source = closure_source("", "").replace(
        "if let Ok(permit) = self.gate.try_lock_owned() {",
        "self.close_runtime_pty_aliases_admitted(); if let Ok(permit) = self.gate.try_lock_owned() {",
    );
    fixture.write(contract::RUNTIME_ALIAS_CLOSURE_PATH, source);
    assert!(
        messages(&fixture)
            .iter()
            .any(|message| message.contains(BYPASS_POLICY))
    );
}

#[test]
fn rejects_admitted_close_from_another_worker_source() {
    let fixture = Fixture::new();
    fixture.write(
        contract::RUNTIME_ALIAS_CLOSURE_PATH,
        closure_source("self.close_runtime_pty_aliases_admitted();", ""),
    );
    fixture.write(
        "core/crates/runtime/engine/worker/src/future_alias_bypass.rs",
        "impl WorkerState { fn bypass(&self) { self.close_runtime_pty_aliases_admitted(); } }\n",
    );
    assert!(
        messages(&fixture)
            .iter()
            .any(|message| message.contains(BYPASS_POLICY))
    );
}

#[test]
fn rejects_unretained_or_branch_external_try_lock_admission() {
    let outside = closure_source("", "").replace(
        "if let Ok(permit) = self.gate.try_lock_owned() {",
        "self.gate.try_lock_owned(); if true { self.close_runtime_pty_aliases_admitted(); } if let Ok(permit) = self.gate.try_lock_owned() {",
    );
    let released = closure_source(
        "drop(permit); self.close_runtime_pty_aliases_admitted();",
        "",
    );
    for source in [outside, released] {
        let fixture = Fixture::new();
        fixture.write(contract::RUNTIME_ALIAS_CLOSURE_PATH, source);
        assert!(
            messages(&fixture)
                .iter()
                .any(|message| message.contains(BYPASS_POLICY))
        );
    }
}

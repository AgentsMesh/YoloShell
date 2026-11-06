use super::fixture::Fixture;

pub fn messages(fixture: &Fixture) -> Vec<String> {
    let files = fixture.sources();
    let mut violations = Vec::new();
    yolo_architecture_process_signal_policy::inspect_repository(&files, &mut violations);
    for file in &files {
        yolo_architecture_process_signal_policy::inspect(file, &mut violations);
    }
    violations.sort();
    violations.dedup();
    violations
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

pub fn write_contract(fixture: &Fixture) {
    fixture.write(
        "core/crates/runtime/engine/process/src/live/live_identity_control.rs",
        r#"
impl LiveProcessIdentity {
    fn signal_authorized(&self) {}
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    fn signal_for_test(&self) {
        self.signal_authorized();
        ProcessSignalAuthority::for_test();
    }
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    fn signal(&self) { self.signal_for_test(); }
}
"#,
    );
    fixture.write(
        "core/crates/runtime/engine/process/src/signal/signal_authority.rs",
        r#"
struct DestructiveSessionAccess;
struct DestructiveSessionAuthority(ProcessSignalAuthority);
struct ForegroundSignalAuthority(ProcessSignalAuthority);
struct ProcessSignalAuthority;
impl DestructiveSessionAccess {
    fn capture() {
        DestructiveSessionAuthority(ProcessSignalAuthority::new());
    }
    fn install_linux_containment() {
        DestructiveSessionAuthority(ProcessSignalAuthority::new());
    }
    #[cfg(test)]
    fn authorize_for_test() {
        DestructiveSessionAuthority(ProcessSignalAuthority::new());
    }
}
impl DestructiveSessionAuthority {
    fn signal(&self) { self.0.signal(); }
}
impl ForegroundSignalAuthority {
    fn verified() { ProcessSignalAuthority::new(); }
    fn signal(&self) { self.0.signal(); }
}
impl ProcessSignalAuthority {
    fn new() {}
    fn signal(&self) { identity.signal_authorized(); }
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    fn for_test() {}
}
fn signal_exact_process() { ProcessSignalAuthority::new().signal(); }
"#,
    );
    fixture.write(
        "core/crates/runtime/engine/process/src/inventory/inventory.rs",
        r#"
struct ProcessIdentity;
struct ProcessGroupIdentity { authority: ForegroundSignalAuthority }
impl ProcessIdentity {
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    fn signal(&self) {}
}
impl ProcessGroupIdentity {
    fn verified() { ForegroundSignalAuthority::verified(); }
}
fn signal_process_group() { authority.signal(); }
"#,
    );
    fixture.write(
        "core/crates/runtime/engine/process/src/captured/captured_targets.rs",
        r#"
fn signal(authority: &DestructiveSessionAuthority) { authority.signal(); }
#[cfg(test)]
fn signal_for_test() {}
"#,
    );
    fixture.write(
        "core/crates/runtime/engine/process/src/inventory/inventory_scope.rs",
        r#"
impl PtyProcessScope {
    fn foreground_group() { ProcessGroupIdentity::verified(); }
}
"#,
    );
}

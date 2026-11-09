use yolo_architecture_foundation::SourceFile;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_syntax::rust_source::RustSource;

const FUTURE_PATH: &str = "core/crates/runtime/engine/process/src/future.rs";
const OBSERVER_PATH: &str = "core/crates/runtime/engine/process/identity/src/process.rs";

fn source(relative: &str, text: &str) -> SourceFile {
    SourceFile {
        absolute: relative.into(),
        relative: relative.into(),
        text: text.into(),
    }
}

#[test]
fn bare_primitive_names_are_not_foreign_definitions_but_direct_extern_items_are() {
    for (text, rejected) in [
        ("kill", false),
        ("extern \"C\" fn kill(pid: i32) {}", true),
        ("fn kill(pid: i32) {}", false),
    ] {
        let file = source(FUTURE_PATH, text);
        let mut violations = Vec::new();
        crate::process_signal_primitives::inspect(&file, &RustSource::parse(text), &mut violations);
        assert_eq!(!violations.is_empty(), rejected, "{text}");
    }
}

#[test]
fn signal_zero_observation_requires_a_complete_final_argument() {
    for text in [
        "fn is_process_alive(pid: i32) { libc::kill(pid); }",
        "fn is_process_alive(pid: i32) { libc::kill(pid, 0; }",
    ] {
        let file = source(OBSERVER_PATH, text);
        let mut violations = Vec::new();
        crate::process_signal_primitives::inspect(&file, &RustSource::parse(text), &mut violations);
        assert!(!violations.is_empty(), "{text}");
    }
}

#[test]
fn syscall_fixture_exemptions_require_positive_test_guards() {
    for (guard, rejected) in [("#[cfg(test)]", false), ("#[cfg(not(test))]", true)] {
        let text =
            format!("{guard} fn fixture() {{ libc::syscall(libc::SYS_tkill, pid, signal); }}");
        let file = source(FUTURE_PATH, &text);
        let mut violations = Vec::new();
        crate::process_signal_syscalls::inspect(&file, &RustSource::parse(&text), &mut violations);
        assert_eq!(!violations.is_empty(), rejected, "{text}");
    }
}

#[test]
fn syscall_constants_cannot_escape_the_approved_call_expression() {
    let text = format!("const BYPASS: i64 = libc::{};", contract::TKILL_SYSCALL);
    let file = source(FUTURE_PATH, &text);
    let mut violations = Vec::new();
    crate::process_signal_syscalls::inspect(&file, &RustSource::parse(&text), &mut violations);
    assert_eq!(violations.len(), 1);
}

#[test]
fn tgkill_observation_rejects_a_single_but_unapproved_signal_identifier() {
    let text = format!(
        "fn {}() {{ libc::syscall(libc::{}, process_id, thread_id, signal); }}",
        contract::THREAD_BELONGS_TO_PROCESS,
        contract::TGKILL_SYSCALL
    );
    let file = source(contract::BOOTSTRAP_TRACE_PATH, &text);
    let mut violations = Vec::new();
    crate::process_signal_syscalls::inspect(&file, &RustSource::parse(&text), &mut violations);
    assert!(!violations.is_empty());
}

#[test]
fn authorized_entry_definitions_and_references_are_bound_to_the_private_owner() {
    for text in [
        "impl LiveProcessIdentity { fn signal_authorized(&self) {} }",
        "fn leak() { let leaked = identity.signal_authorized; }",
    ] {
        let file = source(FUTURE_PATH, text);
        let mut violations = Vec::new();
        crate::process_signal_authorities::inspect(
            &file,
            &RustSource::parse(text),
            &mut violations,
        );
        assert!(
            violations
                .iter()
                .any(|item| item.message == contract::AUTHORIZED_ENTRY_MESSAGE),
            "{text}"
        );
    }
}

#[test]
fn unrelated_signal_methods_in_test_sources_do_not_gain_production_authority_checks() {
    let text = "fn fixture() { unrelated.signal(value, notification); }";
    let file = source(
        "core/crates/runtime/engine/process/src/tests/fixture.rs",
        text,
    );
    let mut violations = Vec::new();
    crate::process_signal_authorities::inspect(&file, &RustSource::parse(text), &mut violations);
    assert!(violations.is_empty(), "{violations:?}");
}

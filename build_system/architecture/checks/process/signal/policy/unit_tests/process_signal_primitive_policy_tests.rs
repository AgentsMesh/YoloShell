use super::fixture::Fixture;
use super::process_signal_test_support::messages;
use yolo_architecture_policy_contracts::process_signal::{
    AUDIT_SIGNAL_FUNCTION, AUDIT_SIGNAL_SYMBOL, BOOTSTRAP_TRACE_PATH, NO_SIGNAL_CONSTANT,
    TGKILL_SYSCALL, THREAD_BELONGS_TO_PROCESS,
};

const FUTURE_PATH: &str = "core/crates/runtime/engine/process/src/future_signal.rs";
const IDENTITY_PATH: &str = "core/crates/runtime/engine/process/identity/src/process.rs";
const PIDFD_PATH: &str =
    "core/crates/runtime/engine/process/src/live/live_identity/linux_pidfd/kernel.rs";
const RAW_POLICY: &str = "raw process signal primitive";
const PIDFD_POLICY: &str = "pidfd signal syscall";
const AUDIT_POLICY: &str = "macOS audit-token signal primitive";

#[test]
fn rejects_raw_libc_and_raw_signal_syscalls() {
    let fixture = Fixture::new();
    fixture.write(
        FUTURE_PATH,
        r#"
fn destroy(pid: i32) {
    unsafe { libc::kill(pid, libc::SIGKILL); }
    let indirect = libc::kill;
    unsafe { libc::r#kill(pid, libc::SIGKILL); }
    unsafe { libc::killpg(pid, libc::SIGTERM); }
    unsafe { libc::syscall(libc::SYS_kill, pid, libc::SIGKILL); }
}
"#,
    );
    let result = messages(&fixture);
    assert_eq!(matches(&result, RAW_POLICY), 5);
}

#[test]
fn lifetime_syntax_cannot_hide_a_raw_primitive() {
    let fixture = Fixture::new();
    fixture.write(
        FUTURE_PATH,
        "fn bypass<'a>(value: &'a i32) { libc::kill(*value, libc::SIGKILL); }\n",
    );
    assert_eq!(matches(&messages(&fixture), RAW_POLICY), 1);
}

#[test]
fn rejects_raw_pidfd_syscall_outside_the_kernel_sender() {
    let fixture = Fixture::new();
    fixture.write(
        FUTURE_PATH,
        "fn bypass() { libc::syscall(libc::SYS_pidfd_send_signal, fd, 9, info, flags); }\n",
    );
    let result = messages(&fixture);
    assert_eq!(matches(&result, PIDFD_POLICY), 1);
}

#[test]
fn observation_owner_cannot_change_signal_zero_to_a_destructive_signal() {
    let allowed = Fixture::new();
    allowed.write(
        IDENTITY_PATH,
        "fn is_process_alive(pid: i32) { libc::kill(pid, 0); }\n",
    );
    assert!(messages(&allowed).is_empty());

    let destructive = Fixture::new();
    destructive.write(
        IDENTITY_PATH,
        "fn is_process_alive(pid: i32) { libc::kill(pid, libc::SIGKILL); }\n",
    );
    assert_eq!(matches(&messages(&destructive), RAW_POLICY), 1);
}

#[test]
fn accepts_only_the_exact_pidfd_kernel_sender() {
    let fixture = Fixture::new();
    fixture.write(
        PIDFD_PATH,
        r#"
impl PidfdOperations for KernelPidfdOperations {
    fn send(&self) {
        libc::syscall(libc::SYS_pidfd_send_signal, fd, signal, info, flags);
    }
}
"#,
    );
    assert!(messages(&fixture).is_empty());
}

#[test]
fn tgkill_observation_requires_no_signal_in_the_exact_owner() {
    let allowed = Fixture::new();
    allowed.write(
        BOOTSTRAP_TRACE_PATH,
        format!(
            "fn {THREAD_BELONGS_TO_PROCESS}() {{\n\
             libc::syscall(libc::{TGKILL_SYSCALL}, process_id, thread_id, {NO_SIGNAL_CONSTANT});\n\
             }}\n"
        ),
    );
    assert!(messages(&allowed).is_empty());

    let destructive = Fixture::new();
    destructive.write(
        BOOTSTRAP_TRACE_PATH,
        format!(
            "fn {THREAD_BELONGS_TO_PROCESS}() {{\n\
             libc::syscall(libc::{TGKILL_SYSCALL}, process_id, thread_id, libc::SIGKILL);\n\
             }}\n"
        ),
    );
    assert_eq!(matches(&messages(&destructive), RAW_POLICY), 1);
}

#[test]
fn tgkill_requires_no_signal_as_the_exact_final_argument() {
    let fixture = Fixture::new();
    fixture.write(
        BOOTSTRAP_TRACE_PATH,
        format!(
            "fn {THREAD_BELONGS_TO_PROCESS}() {{\n\
             libc::syscall(libc::{TGKILL_SYSCALL}, process_id + {NO_SIGNAL_CONSTANT}, thread_id, libc::SIGKILL);\n\
             }}\n"
        ),
    );
    assert_eq!(matches(&messages(&fixture), RAW_POLICY), 1);
}

#[test]
fn grouped_glob_and_namespace_aliases_cannot_hide_raw_primitives() {
    for source in [
        "use libc::kill;\nfn bypass() { kill(pid, signal); }\n",
        "use libc::{kill as terminate};\nfn bypass() { terminate(pid, signal); }\n",
        "use libc::*;\nfn bypass() { kill(pid, signal); }\n",
        "use libc as raw;\nfn bypass() { raw::kill(pid, signal); }\n",
        "use libc::{self as raw};\nfn bypass() { raw::kill(pid, signal); }\n",
    ] {
        let fixture = Fixture::new();
        fixture.write(FUTURE_PATH, source);
        assert_ne!(matches(&messages(&fixture), RAW_POLICY), usize::default());
    }
}

#[test]
fn macro_arguments_cannot_select_a_raw_signal_primitive() {
    let fixture = Fixture::new();
    fixture.write(
        FUTURE_PATH,
        r#"
macro_rules! invoke { ($name:ident) => { libc::$name(pid, signal) } }
fn bypass() { invoke!(kill); }
"#,
    );
    assert_eq!(matches(&messages(&fixture), RAW_POLICY), 1);
}

#[test]
fn ptrace_ffi_and_symbol_literals_cannot_hide_raw_primitives() {
    for source in [
        "fn bypass() { libc::ptrace(request, process, address, data); }\n",
        "unsafe extern \"C\" { fn kill(process: i32, signal: i32) -> i32; }\n",
        "const RAW_SYMBOL: &str = \"kill\";\n",
    ] {
        let fixture = Fixture::new();
        fixture.write(FUTURE_PATH, source);
        assert_ne!(matches(&messages(&fixture), RAW_POLICY), usize::default());
    }
}

#[test]
fn rejects_audit_token_signal_primitive_when_moved() {
    let fixture = Fixture::new();
    fixture.write(
        FUTURE_PATH,
        format!(
            r#"
type {AUDIT_SIGNAL_FUNCTION} = unsafe extern "C" fn(i32);
const SYMBOL: &str = "{AUDIT_SIGNAL_SYMBOL}";
"#
        ),
    );
    let result = messages(&fixture);
    assert_eq!(matches(&result, AUDIT_POLICY), 2);
}

fn matches(messages: &[String], expected: &str) -> usize {
    messages
        .iter()
        .filter(|message| message.contains(expected))
        .count()
}

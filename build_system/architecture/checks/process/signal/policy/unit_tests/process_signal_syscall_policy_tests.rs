use super::fixture::Fixture;
use super::process_signal_test_support::messages;

const FUTURE_PATH: &str = "core/crates/runtime/engine/process/src/future_syscall.rs";
const PIDFD_PATH: &str =
    "core/crates/runtime/engine/process/src/live/live_identity/linux_pidfd/kernel.rs";
const OBSERVER_PATH: &str = "core/crates/runtime/engine/worker/src/bootstrap/bootstrap_exec_observer.rs";
const EXEC_POLICY: &str = "raw process exec primitive";
const RAW_POLICY: &str = "raw process signal primitive";
const EXACT_EXECVEAT: &str = r#"
fn kernel_execveat() {
    libc::syscall(
        libc::SYS_execveat,
        runtime_descriptor,
        EMPTY_PATH.as_ptr(),
        pointers.as_ptr(),
        environ.cast_const(),
        libc::AT_EMPTY_PATH,
    );
}
"#;

#[test]
fn rejects_numeric_unknown_aliased_and_wrong_site_syscalls() {
    for source in [
        "fn bypass() { libc::syscall(37, pid, 9); }\n",
        "fn bypass() { libc::syscall(libc::SYS_clone, flags); }\n",
        "fn bypass() { libc::syscall(libc::SYS_kill, pid, 9); }\n",
        "const RAW: libc::c_long = libc::SYS_kill;\n",
        "use libc::syscall;\nfn bypass() { syscall(37, pid, 9); }\n",
        "fn bypass() { unsafe { (libc::syscall)(37, pid, 9); } }\n",
        "fn bypass() { let invoke = libc::syscall; unsafe { invoke(37, pid, 9); } }\n",
        "macro_rules! invoke { ($number:expr) => { libc::syscall($number, pid, 9) } }\nfn bypass() { invoke!(37); }\n",
        "macro_rules! invoke { ($name:ident) => { libc::$name(37, pid, 9) } }\nfn bypass() { invoke!(syscall); }\n",
    ] {
        let fixture = Fixture::new();
        fixture.write(FUTURE_PATH, source);
        assert!(contains(&messages(&fixture), RAW_POLICY), "{source}");
    }
    let wrong_site = Fixture::new();
    wrong_site.write(
        PIDFD_PATH,
        "impl PidfdOperations for KernelPidfdOperations { fn send(&self) { libc::syscall(libc::SYS_gettid); } }\n",
    );
    assert!(contains(&messages(&wrong_site), RAW_POLICY));
}

#[test]
fn accepts_exact_non_signal_syscalls_and_rejects_execveat() {
    let fixture = Fixture::new();
    fixture.write(
        PIDFD_PATH,
        "impl PidfdOperations for KernelPidfdOperations { fn open(&self) { libc::syscall(libc::SYS_pidfd_open, pid, flags); } }\n",
    );
    fixture.write(
        OBSERVER_PATH,
        "fn current_exec_thread_id() { libc::syscall(libc::SYS_gettid); }\n",
    );
    fixture.write(FUTURE_PATH, EXACT_EXECVEAT);
    assert!(contains(&messages(&fixture), EXEC_POLICY));
}

#[test]
fn rejects_execveat_syscall_in_every_file() {
    for source in [
        EXACT_EXECVEAT,
        r#"fn bypass() { libc::syscall(libc::SYS_execveat, runtime_descriptor, EMPTY_PATH.as_ptr(), pointers.as_ptr(), environ.cast_const(), libc::AT_EMPTY_PATH); }"#,
    ] {
        let fixture = Fixture::new();
        fixture.write(FUTURE_PATH, source);
        assert!(contains(&messages(&fixture), EXEC_POLICY), "{source}");
    }
}

#[test]
fn rejects_every_mutation_of_the_descriptor_bound_execveat_shape() {
    for source in [
        r#"fn kernel_execveat() { libc::syscall(libc::SYS_execveat, other_descriptor, EMPTY_PATH.as_ptr(), pointers.as_ptr(), environ.cast_const(), libc::AT_EMPTY_PATH); }"#,
        r#"fn kernel_execveat() { libc::syscall(libc::SYS_execveat, runtime_descriptor, path.as_ptr(), pointers.as_ptr(), environ.cast_const(), libc::AT_EMPTY_PATH); }"#,
        r#"fn kernel_execveat() { libc::syscall(libc::SYS_execveat, runtime_descriptor, EMPTY_PATH.as_ptr(), other.as_ptr(), environ.cast_const(), libc::AT_EMPTY_PATH); }"#,
        r#"fn kernel_execveat() { libc::syscall(libc::SYS_execveat, runtime_descriptor, EMPTY_PATH.as_ptr(), pointers.as_ptr(), environment, libc::AT_EMPTY_PATH); }"#,
        r#"fn kernel_execveat() { libc::syscall(libc::SYS_execveat, runtime_descriptor, EMPTY_PATH.as_ptr(), pointers.as_ptr(), environ.cast_const(), flags); }"#,
        r#"fn kernel_execveat() { libc::syscall(libc::SYS_execveat, runtime_descriptor, EMPTY_PATH.as_ptr(), pointers.as_ptr(), environ.cast_const(), libc::AT_EMPTY_PATH, extra); }"#,
    ] {
        let fixture = Fixture::new();
        fixture.write(FUTURE_PATH, source);
        assert!(contains(&messages(&fixture), EXEC_POLICY), "{source}");
    }
}

#[test]
fn rejects_direct_imported_aliased_foreign_and_string_execveat_bypasses() {
    for source in [
        "fn bypass() { libc::execveat(fd, path, argv, env, flags); }\n",
        "use libc::execveat;\nfn bypass() { execveat(fd, path, argv, env, flags); }\n",
        "fn bypass() { let invoke = libc::syscall; invoke(libc::SYS_execveat, fd); }\n",
        "unsafe extern \"C\" { fn execveat(fd: i32); }\n",
        "const SYMBOL: &str = \"execveat\";\n",
    ] {
        let fixture = Fixture::new();
        fixture.write(FUTURE_PATH, source);
        assert!(contains(&messages(&fixture), EXEC_POLICY), "{source}");
    }
}

fn contains(messages: &[String], expected: &str) -> bool {
    messages.iter().any(|message| message.contains(expected))
}

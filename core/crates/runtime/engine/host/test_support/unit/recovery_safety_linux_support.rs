use crate::host::tests::*;
use std::io::Write;

const LINUX_CONTAINMENT_CAPABILITY: &str = "yolo-linux-recovery-test-capability";
const LINUX_CONTAINMENT_DEVICE: &str = "/dev/null";
const LINUX_DESCENDANT_COMMAND: &str = "IFS= read -r _; (trap '' HUP TERM; while :; do sleep 1; done) & echo $! > \"$YOLO_TEST_DESCENDANT_PID_FILE\"";
const LINUX_DESCENDANT_PID_ENVIRONMENT: &str = "YOLO_TEST_DESCENDANT_PID_FILE";
const LINUX_DESCENDANT_PID_FILE: &str = "descendant.pid";
const LINUX_GRACEFUL_RESISTANT_COMMAND: &str = "trap '' HUP TERM; while :; do sleep 1; done";
const LINUX_SHELL: &str = "sh";
const LINUX_SHELL_COMMAND_ARGUMENT: &str = "-c";
const LINUX_TEST_POLL_INTERVAL: Duration = Duration::from_millis(10);
const LINUX_TEST_TIMEOUT: Duration = Duration::from_secs(5);

fn spawn_containment_root() -> std::process::Child {
    crate::process::spawn_waitable_test_child(
        containment_command(LINUX_GRACEFUL_RESISTANT_COMMAND)
            .stdin(std::process::Stdio::null()),
    )
}

fn spawn_containment_descendant(path: &Path) -> std::process::Child {
    let mut command = containment_command(LINUX_DESCENDANT_COMMAND);
    command
        .env(LINUX_DESCENDANT_PID_ENVIRONMENT, path)
        .stdin(std::process::Stdio::piped());
    crate::process::spawn_waitable_test_child(command)
}

fn containment_command(script: &str) -> std::process::Command {
    let mut command = std::process::Command::new(LINUX_SHELL);
    command
        .args([LINUX_SHELL_COMMAND_ARGUMENT, script])
        .env(
            process::PTY_SESSION_CAPABILITY_ENV,
            LINUX_CONTAINMENT_CAPABILITY,
        )
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    command
}

fn process_identity(child: &std::process::Child) -> process::ProcessIdentity {
    process::ProcessIdentity {
        pid: child.id(),
        start_time: process::process_start_time(child.id()).unwrap(),
    }
}

fn linux_recovery_runtime(
    host: &Host,
    manifest: &TerminalManifest,
    root: process::ProcessIdentity,
) -> RuntimeRecord {
    let mut runtime = host_runtime_record(
        host,
        manifest.terminal_id,
        host.paths.worker_socket(manifest.terminal_id).unwrap(),
    );
    runtime.worker_pid = ZOMBIE_TEST_MISSING_WORKER_PID;
    runtime.worker_start_time = ZOMBIE_TEST_MISSING_WORKER_START_TIME;
    runtime.child_pid = root.pid;
    runtime.child_start_time = root.start_time;
    runtime
}

fn persist_test_linux_containment(
    host: &Host,
    broker: process::linux_containment::LinuxContainmentBroker,
    runtime: &RuntimeRecord,
    root: process::ProcessIdentity,
) {
    let mut lease = broker
        .prepare(runtime.terminal_id, runtime.runtime_epoch)
        .unwrap();
    lease.migrate_bootstrap(root).unwrap();
    let proof = process::PtySessionProof::capture(
        PathBuf::from(LINUX_CONTAINMENT_DEVICE),
        LINUX_CONTAINMENT_CAPABILITY.to_owned(),
    )
    .unwrap();
    lease.bind_pty_proof(&proof).unwrap();
    let path = process::linux_containment::record_path(
        &host.paths.session_dir(runtime.terminal_id),
        runtime.runtime_epoch,
    );
    host.write_namespace_json(&path, lease.record()).unwrap();
}

fn release_containment_descendant(root: &mut std::process::Child) {
    root.stdin
        .as_mut()
        .unwrap()
        .write_all(b"continue\n")
        .unwrap();
}

fn wait_for_descendant_identity(path: &Path) -> process::ProcessIdentity {
    let deadline = std::time::Instant::now() + LINUX_TEST_TIMEOUT;
    loop {
        if let Ok(pid) = std::fs::read_to_string(path).map(|value| value.trim().parse::<u32>())
            && let Ok(pid) = pid
            && let Ok(start_time) = process::process_start_time(pid)
        {
            return process::ProcessIdentity { pid, start_time };
        }
        assert!(
            std::time::Instant::now() < deadline,
            "{ZOMBIE_WAIT_TIMEOUT_DETAIL}"
        );
        std::thread::sleep(LINUX_TEST_POLL_INTERVAL);
    }
}

fn wait_for_process_gone(identity: process::ProcessIdentity) {
    let deadline = std::time::Instant::now() + LINUX_TEST_TIMEOUT;
    while !process_identity_is_gone(identity.pid, identity.start_time) {
        assert!(
            std::time::Instant::now() < deadline,
            "{ZOMBIE_WAIT_TIMEOUT_DETAIL}"
        );
        std::thread::sleep(LINUX_TEST_POLL_INTERVAL);
    }
}

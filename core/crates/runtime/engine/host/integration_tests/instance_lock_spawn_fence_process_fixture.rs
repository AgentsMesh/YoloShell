use crate::host::tests::*;
use std::os::unix::process::CommandExt;

#[test]
fn instance_lock_spawn_fence_process_fixture() {
    match std::env::var(INSTANCE_FENCE_ROLE_ENV).as_deref() {
        Ok(INSTANCE_FENCE_HOLDER_ROLE) => run_instance_fence_holder(),
        Ok(INSTANCE_FENCE_WORKER_ROLE) => run_instance_fence_worker(),
        Ok(INSTANCE_FENCE_LEAF_ROLE) => run_instance_fence_leaf(),
        _ => {}
    }
}

fn run_instance_fence_holder() {
    let root = instance_fence_root();
    let ownership = HostInstanceOwnership::new([
        InstanceLock::acquire(
            &root.join(INSTANCE_FENCE_HOME_LOCK),
            INSTANCE_FENCE_HOME_LABEL,
        )
        .unwrap(),
        InstanceLock::acquire(
            &root.join(INSTANCE_FENCE_ENDPOINT_LOCK),
            INSTANCE_FENCE_ENDPOINT_LABEL,
        )
        .unwrap(),
    ]);
    #[cfg(not(target_os = "macos"))]
    let descriptors = ownership.worker_lock_descriptors_for_spawn().unwrap();
    #[cfg(target_os = "macos")]
    let hook = native_worker_spawn_test_support::install(clear_instance_fence_close_on_exec);
    let command = instance_fence_command(INSTANCE_FENCE_WORKER_ROLE, &root);
    #[cfg(not(target_os = "macos"))]
    let command = {
        let mut command = command;
        // SAFETY: the child hook performs only async-signal-safe fcntl operations.
        unsafe {
            command.pre_exec(move || clear_instance_fence_close_on_exec(descriptors));
        }
        command
    };
    let mut worker = spawn_host_worker_command_before(
        &ownership,
        command,
        std::time::Instant::now() + HOST_BLOCKING_TEST_TIMEOUT,
    )
    .unwrap();
    #[cfg(target_os = "macos")]
    hook.assert_called();
    worker.wait().unwrap();
}

fn run_instance_fence_worker() {
    let root = instance_fence_root();
    let mut leaf = crate::process::spawn_waitable_test_child(instance_fence_command(
        INSTANCE_FENCE_LEAF_ROLE,
        &root,
    ));
    let worker = process::ProcessIdentity::capture(std::process::id()).unwrap();
    let leaf_identity = process::ProcessIdentity::capture(leaf.id()).unwrap();
    let evidence = InstanceFenceEvidence {
        worker_pid: worker.pid,
        worker_start_time: worker.start_time,
        leaf_pid: leaf_identity.pid,
        leaf_start_time: leaf_identity.start_time,
    };
    atomic_write_json(&root.join(INSTANCE_FENCE_EVIDENCE), &evidence).unwrap();
    leaf.wait().unwrap();
}

fn instance_fence_command(role: &str, root: &Path) -> std::process::Command {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            crate::constants::TEST_HARNESS_EXACT_ARGUMENT,
            INSTANCE_FENCE_FIXTURE_TEST,
            crate::constants::TEST_HARNESS_NOCAPTURE_ARGUMENT,
            crate::constants::TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
        ])
        .env(INSTANCE_FENCE_ROLE_ENV, role)
        .env(INSTANCE_FENCE_ROOT_ENV, root)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit());
    command
}

fn verified_instance_fence_group(identity: process::ProcessIdentity) -> i32 {
    let process_group = i32::try_from(identity.pid).unwrap();
    assert_eq!(unsafe { libc::getpgid(process_group) }, process_group);
    process_group
}

include!("../test_support/integration/instance_lock_spawn.rs");

fn wait_instance_fence_evidence(root: &Path) -> InstanceFenceEvidence {
    let path = root.join(INSTANCE_FENCE_EVIDENCE);
    let deadline = std::time::Instant::now() + HOST_BLOCKING_TEST_TIMEOUT;
    loop {
        if let Ok(evidence) = read_json(&path) {
            return evidence;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "{INSTANCE_FENCE_WAIT_ERROR}"
        );
        std::thread::sleep(PROCESS_EXIT_POLL_INTERVAL);
    }
}

fn wait_instance_fence_exit(identity: process::ProcessIdentity) {
    assert!(
        wait_instance_fence_identity(identity),
        "{INSTANCE_FENCE_WAIT_ERROR}"
    );
}

fn wait_instance_fence_identity(identity: process::ProcessIdentity) -> bool {
    let deadline = std::time::Instant::now() + HOST_BLOCKING_TEST_TIMEOUT;
    while identity.matches() {
        if std::time::Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(PROCESS_EXIT_POLL_INTERVAL);
    }
    true
}

fn wait_instance_fence_child(child: &mut std::process::Child) -> Option<std::process::ExitStatus> {
    let deadline = std::time::Instant::now() + HOST_BLOCKING_TEST_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(PROCESS_EXIT_POLL_INTERVAL);
            }
            Ok(None) | Err(_) => return None,
        }
    }
}

fn signal_instance_fence(identity: process::ProcessIdentity) {
    let _ = identity.signal(libc::SIGKILL);
}

fn signal_instance_fence_group(identity: process::ProcessIdentity, process_group: i32) {
    if let Ok(group) = process::PtyProcessScope::capture(identity)
        .and_then(|scope| scope.foreground_group(process_group))
    {
        let _ = process::signal_process_group(group, libc::SIGKILL);
    }
}

fn request_instance_fence_shutdown(root: &Path) {
    let _ = std::fs::File::create(root.join(INSTANCE_FENCE_SHUTDOWN));
}

fn wait_instance_fence_group(identity: process::ProcessIdentity, process_group: i32) -> bool {
    let deadline = std::time::Instant::now() + HOST_BLOCKING_TEST_TIMEOUT;
    while instance_fence_group_exists(identity, process_group) {
        if std::time::Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(PROCESS_EXIT_POLL_INTERVAL);
    }
    true
}

fn instance_fence_group_exists(identity: process::ProcessIdentity, process_group: i32) -> bool {
    process::PtyProcessScope::capture(identity)
        .and_then(|scope| scope.foreground_group(process_group))
        .is_ok()
}

fn run_instance_fence_leaf() {
    let shutdown = instance_fence_root().join(INSTANCE_FENCE_SHUTDOWN);
    while !shutdown.exists() {
        std::thread::sleep(PROCESS_EXIT_POLL_INTERVAL);
    }
}

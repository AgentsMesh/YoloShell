use super::*;

const HUP_IGNORING_COMMAND: &[u8] =
    b"trap '' HUP; printf 'hup-ignored-ready\\n'; while :; do sleep 1; done\n";
const HUP_IGNORING_MARKER: &[u8] = b"hup-ignored-ready";
const RESUME_RACE_HOLDER: &str = "resume-race-holder";

#[path = "recovery_support.rs"]
mod recovery_support;
use recovery_support::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hard_host_restart_adopts_same_worker_child_and_runtime_epoch() {
    let mut harness = Harness::start().await;
    let id = harness.create_cat("adoption").await.manifest.terminal_id;
    let before: RuntimeRecord = read_json(&harness.paths().runtime_record(id)).unwrap();
    let old_host_pid = harness.client().status().await.unwrap().host_pid;
    harness.restart_host_hard().await;
    let host_status = harness.client().status().await.unwrap();
    assert_ne!(host_status.host_pid, old_host_pid);
    assert!(host_status.controller_generation > before.controller_generation);
    let after = wait_for_restarted_runtime(&harness, id).await;
    assert_eq!(after.worker_pid, before.worker_pid);
    assert_eq!(after.child_pid, before.child_pid);
    assert_eq!(after.runtime_epoch, before.runtime_epoch);
    assert_eq!(after.status, TerminalRuntimeStatus::Running);
    let target = runtime_ref(id, after.runtime_epoch);
    let attachment = harness.client().attach(id, 0).await.unwrap();
    let lease = harness
        .client()
        .acquire_writer(target, "after-restart", None)
        .await
        .unwrap();
    harness
        .client()
        .input(
            target,
            lease.generation,
            &lease.writer_capability,
            b"still-alive\n".to_vec(),
        )
        .await
        .unwrap();
    read_until(attachment, b"still-alive").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn one_worker_crash_does_not_disturb_another_session() {
    let harness = Harness::start().await;
    let host_before = harness.client().status().await.unwrap();
    let first = harness.create_cat("first").await;
    let second = harness.create_cat("second").await;
    let first_runtime = first.runtime.unwrap();
    let second_runtime = second.runtime.unwrap();
    let second_target = runtime_ref(second.manifest.terminal_id, second_runtime.runtime_epoch);
    let first_worker = ProcessIdentity {
        pid: first_runtime.worker_pid,
        start_time: first_runtime.worker_start_time,
    };
    let second_worker = ProcessIdentity {
        pid: second_runtime.worker_pid,
        start_time: second_runtime.worker_start_time,
    };
    let second_child = ProcessIdentity {
        pid: second_runtime.child_pid,
        start_time: second_runtime.child_start_time,
    };
    assert!(first_worker.signal(libc::SIGKILL).unwrap());
    wait_for_identity_gone(first_worker).await;
    discard_incomplete_coverage_profile(first_runtime.worker_pid);
    let first_record =
        read_json(&harness.paths().runtime_record(first.manifest.terminal_id)).unwrap();
    wait_for_crashed_shell_retirement(&harness, &first_record, None).await;
    let host_after = harness.client().status().await.unwrap();
    assert_eq!(host_after.host_pid, host_before.host_pid);
    assert_eq!(
        host_after.controller_generation,
        host_before.controller_generation
    );
    assert!(second_worker.matches());
    assert!(second_child.matches());
    let surviving = harness
        .client()
        .get_session(second.manifest.terminal_id)
        .await
        .unwrap_or_else(|error| {
            panic!(
                "query surviving session after peer worker crash: {error}; host_stderr={}",
                harness.host_stderr_tail()
            )
        })
        .runtime
        .unwrap();
    assert_eq!(surviving.worker_pid, second_runtime.worker_pid);
    assert_eq!(surviving.child_pid, second_runtime.child_pid);
    assert_eq!(surviving.status, TerminalRuntimeStatus::Running);
    let lease = harness
        .client()
        .acquire_writer(second_target, "survivor", None)
        .await
        .unwrap();
    harness
        .client()
        .input(
            second_target,
            lease.generation,
            &lease.writer_capability,
            b"isolated\n".to_vec(),
        )
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn immediate_resume_after_worker_crash_fences_the_surviving_pty_child() {
    let harness = Harness::start().await;
    let session = harness.create_cat("resume-race").await;
    let terminal_id = session.manifest.terminal_id;
    let before: RuntimeRecord = read_json(&harness.paths().runtime_record(terminal_id)).unwrap();
    let target = runtime_ref_from_record(&before);
    let attachment = harness.client().attach(terminal_id, 0).await.unwrap();
    let lease = harness
        .client()
        .acquire_writer(target, RESUME_RACE_HOLDER, None)
        .await
        .unwrap();
    harness
        .client()
        .input(
            target,
            lease.generation,
            &lease.writer_capability,
            HUP_IGNORING_COMMAND.to_vec(),
        )
        .await
        .unwrap();
    read_until(attachment, HUP_IGNORING_MARKER).await;
    let worker = ProcessIdentity {
        pid: before.worker_pid,
        start_time: before.worker_start_time,
    };
    assert!(worker.signal(libc::SIGKILL).unwrap());
    wait_for_identity_gone(worker).await;
    let selected = harness.client().get_session(terminal_id).await.unwrap();
    let selection = exact_recovery_selection(&selected);
    assert_eq!(selection.expected_runtime_epoch, Some(before.runtime_epoch));
    report_recovery_selection(&harness, &selected);

    let error = harness
        .resume_with_preconditions_and_track(selection, true)
        .await
        .unwrap_err();
    let rejection = assert_immediate_resume_rejected(error);
    let after: RuntimeRecord = read_json(&harness.paths().runtime_record(terminal_id)).unwrap();
    assert_eq!(after.runtime_epoch, before.runtime_epoch);
    assert_eq!(after.child_pid, before.child_pid);
    wait_for_crashed_shell_retirement(&harness, &before, Some(&rejection)).await;
    wait_for_identity_gone(worker).await;
    discard_incomplete_coverage_profile(before.worker_pid);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bridge_stdio_forwards_capnp_rpc() {
    let harness = Harness::start().await;
    let mut bridge = ChildGuard::new(
        Command::new(runtime_binary())
            .args(yolo_launch_contract::ProcessRole::Host.arguments())
            .args([cli::BRIDGE_COMMAND, cli::STDIO_ARGUMENT, cli::HOME_ARGUMENT])
            .arg(harness.home.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let request = RequestEnvelope::new(
        "bridge-status",
        method::HOST_STATUS,
        yolo_capnp_value::Value::Null,
    );
    let response = bridge_request(&mut bridge, &request).await;
    assert!(response.ok);
    assert_eq!(response.request_id, "bridge-status");
    assert!(bridge.wait().unwrap().success());
}

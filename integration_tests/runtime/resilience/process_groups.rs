use super::*;
use std::os::unix::fs::PermissionsExt;

const FOREGROUND_SCRIPT: &str = "foreground-agent.sh";
const FOREGROUND_PID_FILE: &str = "foreground-agent.pid";
const PROCESS_TREE_SESSION: &str = "process-tree";
const FOREGROUND_READY: &[u8] = b"foreground-agent-ready";
const FOREGROUND_SCRIPT_CONTENT: &str = "#!/bin/sh\ntrap '' HUP TERM\necho $$ > foreground-agent.pid\necho foreground-agent-ready\nwhile :; do sleep 1; done\n";
const TREE_SCRIPT: &str = "stubborn-tree.sh";
const TREE_AGENT_PID_FILE: &str = "tree-agent.pid";
const TREE_DESCENDANT_PID_FILE: &str = "tree-descendant.pid";
const TREE_READY: &[u8] = b"stubborn-tree-ready";
const TREE_SCRIPT_CONTENT: &str = "#!/bin/sh\ntrap '' HUP TERM\n/bin/sh -c \"trap '' HUP TERM; echo \\$\\$ > tree-descendant.pid; while :; do sleep 1; done\" &\necho $$ > tree-agent.pid\necho stubborn-tree-ready\nwait\n";
const POST_SIGNAL_MARKER: &[u8] = b"shell-survived-foreground-signal";
const PROCESS_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(5);
const PROCESS_DISCOVERY_POLL: Duration = Duration::from_millis(20);
const HUP_SETTLE_DELAY: Duration = Duration::from_millis(100);
const SCRIPT_MODE: u32 = 0o700;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explicit_signal_targets_the_interactive_shell_foreground_group() {
    let harness = Harness::start().await;
    install_script(&harness, FOREGROUND_SCRIPT, FOREGROUND_SCRIPT_CONTENT);
    let session = harness.create_cat("foreground-signal").await;
    let terminal_id = session.manifest.terminal_id;
    let runtime: RuntimeRecord = read_json(&harness.paths().runtime_record(terminal_id)).unwrap();
    let target = runtime_ref_from_record(&runtime);
    let lease = acquire_writer(&harness, target).await;
    let attachment = harness.client().attach(terminal_id, 0).await.unwrap();
    send_command(&harness, target, &lease, FOREGROUND_SCRIPT).await;
    read_until(attachment, FOREGROUND_READY).await;
    let agent = wait_for_pid(harness.home.path().join(FOREGROUND_PID_FILE)).await;

    assert_ne!(process_group(agent.pid), process_group(runtime.child_pid));
    harness
        .client()
        .signal(target, lease.generation, &lease.writer_capability, "INT")
        .await
        .unwrap();
    wait_for_identity_gone(agent).await;
    assert!(process::process_identity_matches(
        runtime.child_pid,
        runtime.child_start_time
    ));

    let attachment = harness.client().attach(terminal_id, 0).await.unwrap();
    harness
        .client()
        .input(
            target,
            lease.generation,
            &lease.writer_capability,
            b"printf 'shell-survived-foreground-signal\\n'\n".to_vec(),
        )
        .await
        .unwrap();
    read_until(attachment, POST_SIGNAL_MARKER).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn force_terminate_reaps_shell_agent_and_descendants_ignoring_hup_and_term() {
    let harness = Harness::start().await;
    let (target, lease, shell, agent, descendant) = start_stubborn_tree(&harness).await;

    assert_ne!(process_group(agent.pid), process_group(shell.pid));
    harness
        .client()
        .signal(target, lease.generation, &lease.writer_capability, "HUP")
        .await
        .unwrap();
    tokio::time::sleep(HUP_SETTLE_DELAY).await;
    assert!(identity_matches(agent));
    assert!(identity_matches(descendant));

    harness
        .client()
        .terminate(target.terminal_id, true)
        .await
        .unwrap();
    wait_for_identity_gone(shell).await;
    wait_for_identity_gone(agent).await;
    wait_for_identity_gone(descendant).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graceful_terminate_escalates_for_term_ignoring_foreground_descendants() {
    let harness = Harness::start().await;
    let (target, _lease, shell, agent, descendant) = start_stubborn_tree(&harness).await;

    assert_ne!(process_group(agent.pid), process_group(shell.pid));
    terminate_stubborn_tree(&harness, target.terminal_id).await;
    wait_for_identity_gone(shell).await;
    wait_for_identity_gone(agent).await;
    wait_for_identity_gone(descendant).await;
}

async fn terminate_stubborn_tree(harness: &Harness, terminal_id: Uuid) {
    let result = harness.client().terminate(terminal_id, false).await;
    match result {
        Ok(()) => {}
        Err(ClientError::Remote { code, .. }) if code == error_code::CLEANUP_DEFERRED => {}
        Err(error) => panic!("unexpected stubborn-tree termination result: {error}"),
    }
}

pub(crate) async fn start_stubborn_tree(
    harness: &Harness,
) -> (
    RuntimeRef,
    WriterLeaseGrant,
    ProcessIdentity,
    ProcessIdentity,
    ProcessIdentity,
) {
    install_script(harness, TREE_SCRIPT, TREE_SCRIPT_CONTENT);
    let session = harness.create_cat(PROCESS_TREE_SESSION).await;
    let terminal_id = session.manifest.terminal_id;
    let runtime: RuntimeRecord = read_json(&harness.paths().runtime_record(terminal_id)).unwrap();
    let target = runtime_ref_from_record(&runtime);
    let lease = acquire_writer(harness, target).await;
    let attachment = harness.client().attach(terminal_id, 0).await.unwrap();
    send_command(harness, target, &lease, TREE_SCRIPT).await;
    read_until(attachment, TREE_READY).await;
    let agent = wait_for_pid(harness.home.path().join(TREE_AGENT_PID_FILE)).await;
    let descendant = wait_for_pid(harness.home.path().join(TREE_DESCENDANT_PID_FILE)).await;
    (
        target,
        lease,
        identity(runtime.child_pid, runtime.child_start_time),
        agent,
        descendant,
    )
}

async fn acquire_writer(harness: &Harness, target: RuntimeRef) -> WriterLeaseGrant {
    harness
        .client()
        .acquire_writer(target, "process-group-test", None)
        .await
        .unwrap()
}

async fn send_command(
    harness: &Harness,
    target: RuntimeRef,
    grant: &WriterLeaseGrant,
    script: &str,
) {
    harness
        .client()
        .input(
            target,
            grant.generation,
            &grant.writer_capability,
            format!("./{script}\n").into_bytes(),
        )
        .await
        .unwrap();
}

fn install_script(harness: &Harness, name: &str, content: &str) {
    let path = harness.home.path().join(name);
    std::fs::write(&path, content).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(SCRIPT_MODE)).unwrap();
}

async fn wait_for_pid(path: PathBuf) -> ProcessIdentity {
    let deadline = tokio::time::Instant::now() + PROCESS_DISCOVERY_TIMEOUT;
    loop {
        if let Ok(text) = std::fs::read_to_string(&path)
            && let Ok(pid) = text.trim().parse()
            && let Some(identity) = ProcessIdentity::capture(pid)
        {
            return identity;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "process PID file was not created: {}",
            path.display()
        );
        tokio::time::sleep(PROCESS_DISCOVERY_POLL).await;
    }
}

fn identity(pid: u32, start_time: u64) -> ProcessIdentity {
    ProcessIdentity { pid, start_time }
}

fn identity_matches(identity: ProcessIdentity) -> bool {
    process::process_identity_matches(identity.pid, identity.start_time)
}

fn process_group(pid: u32) -> i32 {
    let group = unsafe { libc::getpgid(pid as i32) };
    assert!(group > 0, "process group unavailable for PID {pid}");
    group
}

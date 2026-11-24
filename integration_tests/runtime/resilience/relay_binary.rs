use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use yolo_hub_relay_control_protocol::StatusKind;

#[path = "relay_binary_status_fd.rs"]
pub(crate) mod status_fd;
use status_fd::InheritedStatusChannel;

const BAZEL_ATTACH_BINARY_ENV: &str = "YOLO_ATTACH_BINARY";
const COMMAND_MARKER: &[u8] = b"attach-binary-ready";
const HOST_IDENTITY_CAPTURE_CONTEXT: &str = "capture attach-started Host identity";
const OUTPUT_BUFFER_BYTES: usize = 4096;
const PROCESS_TIMEOUT: Duration = Duration::from_secs(5);

fn attach_binary() -> PathBuf {
    std::env::var_os(BAZEL_ATTACH_BINARY_ENV)
        .map(PathBuf::from)
        .expect("Yolo binary path is required for terminal relay")
}

fn attach_command(home: &Path, id: Uuid) -> tokio::process::Command {
    let mut command = tokio::process::Command::new(attach_binary());
    command
        .args(yolo_launch_contract::ProcessRole::Attach.arguments())
        .arg(cli::HOME_ARGUMENT)
        .arg(home)
        .arg(cli::TERMINAL_ID_ARGUMENT)
        .arg(id.to_string())
        .arg(cli::HOLDER_ID_ARGUMENT)
        .arg("binary-holder")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    command
}

async fn read_marker(output: &mut (impl tokio::io::AsyncRead + Unpin)) {
    let deadline = tokio::time::Instant::now() + PROCESS_TIMEOUT;
    let mut received = Vec::new();
    let mut buffer = [0_u8; OUTPUT_BUFFER_BYTES];
    while !received
        .windows(COMMAND_MARKER.len())
        .any(|window| window == COMMAND_MARKER)
    {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        assert!(!remaining.is_zero(), "binary relay output: {received:?}");
        let length = timeout(remaining, output.read(&mut buffer))
            .await
            .unwrap()
            .unwrap();
        assert_ne!(length, 0, "binary relay closed early: {received:?}");
        received.extend_from_slice(&buffer[..length]);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn attach_binary_relays_bytes_and_eof_only_detaches() {
    let harness = Harness::start().await;
    let session = harness.create_cat("attach-binary").await;
    let id = session.manifest.terminal_id;
    let mut child = attach_command(harness.home.path(), id).spawn().unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = child.stdout.take().unwrap();
    input
        .write_all(b"printf 'attach-binary-ready\\n'\n")
        .await
        .unwrap();

    read_marker(&mut output).await;
    drop(input);
    let status = timeout(PROCESS_TIMEOUT, child.wait())
        .await
        .unwrap()
        .unwrap();
    assert!(status.success());
    let view = harness.client().get_session(id).await.unwrap();
    assert_eq!(view.runtime.unwrap().status, TerminalRuntimeStatus::Running);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn attach_binary_starts_host_and_adopts_the_existing_worker() {
    let mut harness = Harness::start().await;
    let session = harness.create_cat("attach-starts-host").await;
    let id = session.manifest.terminal_id;
    let original = session.runtime.unwrap();
    harness.kill_host_hard();

    let mut command = attach_command(harness.home.path(), id);
    command
        .arg(cli::HOST_EXECUTABLE_ARGUMENT)
        .arg(runtime_binary());
    let mut child = command.spawn().unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = child.stdout.take().unwrap();
    input
        .write_all(b"printf 'attach-binary-ready\\n'\n")
        .await
        .unwrap();
    read_marker(&mut output).await;
    drop(input);
    assert!(
        timeout(PROCESS_TIMEOUT, child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );

    let restored = harness
        .client()
        .get_session(id)
        .await
        .unwrap()
        .runtime
        .unwrap();
    assert_eq!(restored.worker_pid, original.worker_pid);
    assert_eq!(restored.child_pid, original.child_pid);
    assert_eq!(restored.runtime_epoch, original.runtime_epoch);
    let host_pid = harness.client().status().await.unwrap().host_pid;
    let host = ProcessIdentity::capture(host_pid).expect(HOST_IDENTITY_CAPTURE_CONTEXT);
    assert!(host.signal(libc::SIGINT).unwrap());
    wait_for_identity_gone(host).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn status_peer_loss_detaches_and_releases_without_stopping_the_session() {
    let harness = Harness::start().await;
    let session = harness.create_cat("attach-status-peer").await;
    let id = session.manifest.terminal_id;
    let target = runtime_ref_from_view(&session);
    let runtime = session.runtime.unwrap();
    let mut command = attach_command(harness.home.path(), id);
    let status_channel = InheritedStatusChannel::prepare(&mut command);
    let mut child = command.spawn().unwrap();
    let mut reader = status_channel.into_peer();
    let status = timeout(PROCESS_TIMEOUT, read_relay_status(&mut reader))
        .await
        .unwrap();
    assert_eq!(status.kind, StatusKind::ControlAcquired);
    assert_eq!(status.holder_id, None);
    drop(reader);

    let exit = timeout(PROCESS_TIMEOUT, child.wait())
        .await
        .unwrap()
        .unwrap();
    assert!(!exit.success());
    let view = harness.client().get_session(id).await.unwrap();
    let surviving = view.runtime.unwrap();
    assert_eq!(surviving.status, TerminalRuntimeStatus::Running);
    assert_eq!(surviving.worker_pid, runtime.worker_pid);
    assert_eq!(surviving.child_pid, runtime.child_pid);
    assert_eq!(
        harness.client().get_writer_lease(target).await.unwrap(),
        None
    );
}

use crate::fake_claude_constants::DATA_HOME_DIRECTORY;
use crate::fake_claude_process::collect_frames_for_test;
use crate::fake_claude_scenarios::lifecycle;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const BLOCKED_READER_WINDOW: Duration = Duration::from_millis(100);
const CHILD_REAP_POLL_INTERVAL: Duration = Duration::from_millis(5);
const CHILD_REAP_TIMEOUT: Duration = Duration::from_secs(5);
const CAT_EXECUTABLE: &str = "/bin/cat";
const MALFORMED_FRAME: &[u8] = b"not-json\n";

#[test]
fn partial_delivery_with_open_socket_is_bounded_and_reaps_the_child() {
    let directory = tempfile::tempdir().unwrap();
    let mut scenario = lifecycle(
        directory.path(),
        &directory.path().join(DATA_HOME_DIRECTORY),
    );
    let frame = scenario.frames.remove(0);
    let (reader, mut writer) = UnixStream::pair().unwrap();
    serde_json::to_writer(&mut writer, &frame).unwrap();
    writer.write_all(b"\n").unwrap();
    let (child, _input, identity) = blocked_child();

    let error =
        collect_frames_for_test(reader, child, Instant::now() + BLOCKED_READER_WINDOW).unwrap_err();

    assert!(matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    ));
    assert_reaped(identity);
}

#[test]
fn malformed_frame_reaps_the_child_without_waiting_for_socket_close() {
    let (reader, mut writer) = UnixStream::pair().unwrap();
    writer.write_all(MALFORMED_FRAME).unwrap();
    let (child, _input, identity) = blocked_child();

    let error =
        collect_frames_for_test(reader, child, Instant::now() + BLOCKED_READER_WINDOW).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert_reaped(identity);
}

fn blocked_child() -> (
    std::process::Child,
    std::process::ChildStdin,
    yolo_runtime::process::ProcessIdentity,
) {
    let mut child = Command::new(CAT_EXECUTABLE)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let identity = yolo_runtime::process::ProcessIdentity::capture(child.id()).unwrap();
    let input = child.stdin.take().unwrap();
    (child, input, identity)
}

fn assert_reaped(identity: yolo_runtime::process::ProcessIdentity) {
    let deadline = Instant::now() + CHILD_REAP_TIMEOUT;
    while identity.matches() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(CHILD_REAP_POLL_INTERVAL);
    }
}

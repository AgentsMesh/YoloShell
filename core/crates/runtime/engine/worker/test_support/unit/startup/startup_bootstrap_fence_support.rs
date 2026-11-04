use super::*;

pub(super) const HELPER_TEST: &str =
    "worker::startup::startup_guard::bootstrap_fence_cleanup_tests::bootstrap_fenced_child_helper";
pub(super) const HELPER_ROLE_ENV: &str = "YOLO_BOOTSTRAP_FENCE_HELPER";
pub(super) const HELPER_ROLE: &str = "exit-before-bootstrap-gate";
pub(super) const HELPER_RELEASE_ENV: &str = "YOLO_BOOTSTRAP_FENCE_RELEASE";
pub(super) const HELPER_EXITING_ENV: &str = "YOLO_BOOTSTRAP_FENCE_EXITING";
pub(super) const HELPER_DESCRIPTOR_SOCKET_ENV: &str = "YOLO_BOOTSTRAP_FENCE_DESCRIPTOR_SOCKET";
pub(super) const RELEASE_FILE: &str = "release";
pub(super) const EXITING_FILE: &str = "exiting";
pub(super) const DESCRIPTOR_SOCKET_FILE: &str = "descriptor.sock";
pub(super) const JOURNAL_MARKER: &str = "bootstrap-fence";
pub(super) const TERMINAL_ROWS: u16 = 24;
pub(super) const TERMINAL_COLUMNS: u16 = 80;
pub(super) const TERMINAL_PIXEL_SIZE: u16 = 0;
pub(super) const HELPER_POLL_INTERVAL: Duration = Duration::from_millis(5);
pub(super) const CLEANUP_TIMEOUT: Duration =
    crate::worker::termination_timing::WORKER_TERMINATION_TIMEOUT;
pub(super) const CHILD_EXIT_TIMEOUT_DETAIL: &str = "pre-gate bootstrap child did not begin exiting";
pub(super) const CLEANUP_TIMEOUT_DETAIL: &str =
    "pre-gate tracked PTY cleanup did not publish completion";
pub(super) const DESCRIPTOR_CONNECTION_TIMEOUT_DETAIL: &str =
    "pre-gate bootstrap child did not request its descriptor capability";
pub(super) const DESCRIPTOR_RECEIVE_TIMEOUT_DETAIL: &str =
    "pre-gate bootstrap child did not receive its descriptor capability";

pub(super) const HELPER_HANGUP_ROLE: &str = "hangup-before-bootstrap-gate";

pub(super) fn receive_descriptor(
    stream: &UnixStream,
) -> crate::process::MacosProcessCapabilityChildWriter {
    let deadline = Instant::now() + CLEANUP_TIMEOUT;
    loop {
        match crate::worker::receive_bootstrap_writer_once_for_test(stream.as_raw_fd()) {
            Ok(writer) => return writer,
            Err(error)
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|error| error.kind() == std::io::ErrorKind::WouldBlock) =>
            {
                assert!(
                    Instant::now() < deadline,
                    "{DESCRIPTOR_RECEIVE_TIMEOUT_DETAIL}"
                );
                std::thread::sleep(HELPER_POLL_INTERVAL);
            }
            Err(error) => panic!("{error:#}"),
        }
    }
}

pub(super) fn bootstrap_helper_command(
    release: &Path,
    exiting: &Path,
    descriptor_socket: &Path,
    capability: &str,
) -> BootstrapSpawnCommand {
    let mut command = CommandBuilder::new(std::env::current_exe().unwrap());
    command.args([
        crate::constants::TEST_HARNESS_EXACT_ARGUMENT,
        HELPER_TEST,
        crate::constants::TEST_HARNESS_NOCAPTURE_ARGUMENT,
        crate::constants::TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
    ]);
    command.env(HELPER_ROLE_ENV, HELPER_ROLE);
    command.env(HELPER_RELEASE_ENV, release);
    command.env(HELPER_EXITING_ENV, exiting);
    command.env(HELPER_DESCRIPTOR_SOCKET_ENV, descriptor_socket);
    command.env(crate::process::PTY_SESSION_CAPABILITY_ENV, capability);
    BootstrapSpawnCommand {
        command,
        gate: UnreleasedBootstrapGate,
    }
}

pub(super) fn accept_descriptor_stream(listener: &UnixListener) -> UnixStream {
    let deadline = Instant::now() + CLEANUP_TIMEOUT;
    loop {
        match listener.accept() {
            Ok((stream, _address)) => return stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(
                    Instant::now() < deadline,
                    "{DESCRIPTOR_CONNECTION_TIMEOUT_DETAIL}"
                );
                std::thread::sleep(HELPER_POLL_INTERVAL);
            }
            Err(error) => panic!("{error}"),
        }
    }
}

pub(super) fn wait_for_marker(marker: &Path) {
    let deadline = Instant::now() + CLEANUP_TIMEOUT;
    while !marker.exists() {
        assert!(Instant::now() < deadline, "{CHILD_EXIT_TIMEOUT_DETAIL}");
        std::thread::sleep(HELPER_POLL_INTERVAL);
    }
}

pub(super) fn pty_size() -> PtySize {
    PtySize {
        rows: TERMINAL_ROWS,
        cols: TERMINAL_COLUMNS,
        pixel_width: TERMINAL_PIXEL_SIZE,
        pixel_height: TERMINAL_PIXEL_SIZE,
    }
}

pub(super) fn interrupted_profile(signal: Option<i32>) -> bool {
    matches!(signal, Some(libc::SIGHUP) | Some(libc::SIGKILL))
}

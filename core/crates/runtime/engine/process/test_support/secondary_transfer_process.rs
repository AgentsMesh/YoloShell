use super::*;
use std::os::fd::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const ACCEPT_FAILURE_DETAIL: &str = "secondary custody descriptor accept failed";
const HELPER_EARLY_EXIT_DETAIL: &str = "secondary custody descriptor helper exited before ready";
const HELPER_POLL_INTERVAL: Duration = Duration::from_millis(10);
const RECEIVE_FAILURE_DETAIL: &str = "receive secondary custody reader";

pub(super) struct Receiver {
    _directory: tempfile::TempDir,
    child: Child,
    ready: PathBuf,
}

impl Receiver {
    pub(super) fn spawn(
        descriptor: std::os::fd::BorrowedFd<'_>,
        identity: MacosProcessCapabilityIdentity,
    ) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join(SOCKET_COMPONENT);
        let ready = directory.path().join(READY_COMPONENT);
        let listener = UnixListener::bind(&socket).unwrap();
        listener.set_nonblocking(true).unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                TEST_HARNESS_EXACT_ARGUMENT,
                HELPER_TEST,
                TEST_HARNESS_NOCAPTURE_ARGUMENT,
                TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
            ])
            .env(HELPER_ROLE_ENV, HELPER_ROLE)
            .env(HELPER_SOCKET_ENV, &socket)
            .env(HELPER_READER_ENV, identity.reader_handle().to_string())
            .env(HELPER_WRITER_ENV, identity.writer_handle().to_string())
            .env(HELPER_READY_ENV, &ready)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = spawn_guarded(&mut command);
        let stream = accept_before_deadline(&listener, &mut child);
        crate::local_unix::test_support::send_right(
            stream.as_raw_fd(),
            descriptor.as_raw_fd(),
            RIGHTS_MARKER,
        );
        Self {
            _directory: directory,
            child,
            ready,
        }
    }

    pub(super) fn wait_ready(&mut self) {
        let deadline = Instant::now() + HELPER_TIMEOUT;
        while !self.ready.exists() {
            if let Some(status) = self.child.try_wait().unwrap() {
                panic!("{HELPER_EARLY_EXIT_DETAIL}: {status}");
            }
            assert!(Instant::now() < deadline, "{HELPER_TIMEOUT_DETAIL}");
            std::thread::sleep(HELPER_POLL_INTERVAL);
        }
    }

    pub(super) fn kill_and_wait(&mut self) {
        self.child.kill().unwrap();
        let _ = self.child.wait().unwrap();
    }
}

fn accept_before_deadline(listener: &UnixListener, child: &mut Child) -> UnixStream {
    let deadline = Instant::now() + HELPER_TIMEOUT;
    loop {
        match listener.accept() {
            Ok((stream, _)) => return stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if let Some(status) = child.try_wait().unwrap() {
                    panic!("{HELPER_EARLY_EXIT_DETAIL}: {status}");
                }
                assert!(Instant::now() < deadline, "{HELPER_TIMEOUT_DETAIL}");
                std::thread::sleep(HELPER_POLL_INTERVAL);
            }
            Err(error) => panic!("{ACCEPT_FAILURE_DETAIL}: {error}"),
        }
    }
}

pub(super) fn spawn_shell_holder(writer: &MacosProcessCapabilityChildWriter) -> Child {
    let mut command = Command::new(SHELL_COMMAND);
    command
        .args(["-c", SHELL_SCRIPT])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    writer.configure_exec_inheritance(&mut command);
    spawn_guarded(&mut command)
}

fn spawn_guarded(command: &mut Command) -> Child {
    let _guard = crate::process::waitable_child_spawn_guard().unwrap();
    command.spawn().unwrap()
}

pub(super) fn required_path(name: &str) -> PathBuf {
    std::env::var_os(name).map(PathBuf::from).unwrap()
}

pub(super) fn required_u64(name: &str) -> u64 {
    std::env::var(name).unwrap().parse().unwrap()
}

pub(super) fn receive_descriptor(stream: &UnixStream) -> std::os::fd::OwnedFd {
    let deadline = Instant::now() + HELPER_TIMEOUT;
    loop {
        match crate::local_unix::macos::receive_exactly_one_right(stream.as_raw_fd(), RIGHTS_MARKER)
        {
            Ok(descriptor) => return descriptor,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline, "{HELPER_TIMEOUT_DETAIL}");
                std::thread::sleep(HELPER_POLL_INTERVAL);
            }
            Err(error) => panic!("{RECEIVE_FAILURE_DETAIL}: {error}"),
        }
    }
}

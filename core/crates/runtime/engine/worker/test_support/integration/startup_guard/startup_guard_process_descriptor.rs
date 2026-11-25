use super::super::StartupChildGuard;
use crate::process::{
    MacosProcessCapabilityChildWriter, MacosProcessCapabilityIssuer,
    MacosProcessCapabilityManagerReader,
};
use portable_pty::CommandBuilder;
use std::os::fd::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

const DESCRIPTOR_SOCKET_ENV: &str = "YOLO_STARTUP_GUARD_DESCRIPTOR_SOCKET";
const DESCENDANT_DESCRIPTOR_ENV: &str = "YOLO_STARTUP_GUARD_DESCENDANT_DESCRIPTOR";
const DESCRIPTOR_READY_EXTENSION: &str = "descriptor-ready";
const DESCRIPTOR_SOCKET_EXTENSION: &str = "descriptor-socket";
const DESCRIPTOR_TIMEOUT: Duration = Duration::from_secs(3);
const DESCRIPTOR_POLL_INTERVAL: Duration = Duration::from_millis(10);
const DESCRIPTOR_ACCEPT_DETAIL: &str = "startup root did not request its process descriptor";
const DESCRIPTOR_RECEIVE_DETAIL: &str = "startup root did not receive its process descriptor";
const DESCRIPTOR_READY_DETAIL: &str = "startup root did not acknowledge its process descriptor";
const FCNTL_FAILURE: libc::c_int = -1;

pub(super) struct ParentDescriptor {
    listener: UnixListener,
    socket: PathBuf,
    ready: PathBuf,
    reader: Arc<MacosProcessCapabilityManagerReader>,
    writer: MacosProcessCapabilityChildWriter,
}

impl ParentDescriptor {
    pub(super) fn stage(marker: &Path) -> Self {
        let socket = marker.with_extension(DESCRIPTOR_SOCKET_EXTENSION);
        let ready = marker.with_extension(DESCRIPTOR_READY_EXTENSION);
        let listener = UnixListener::bind(&socket).unwrap();
        listener.set_nonblocking(true).unwrap();
        let (reader, writer) = MacosProcessCapabilityIssuer::create()
            .unwrap()
            .into_staged_for_test()
            .unwrap();
        Self {
            listener,
            socket,
            ready,
            reader,
            writer,
        }
    }

    pub(super) fn configure_root(&self, command: &mut CommandBuilder) {
        command.env(DESCRIPTOR_SOCKET_ENV, &self.socket);
    }

    pub(super) fn install(self, guard: &mut StartupChildGuard) {
        let stream = accept_before(&self.listener);
        crate::worker::send_bootstrap_writer_once_for_test(
            stream.as_raw_fd(),
            self.writer.descriptor(),
        )
        .unwrap();
        wait_for_path(&self.ready, DESCRIPTOR_READY_DETAIL);
        drop(self.writer);
        guard.install_process_capability(self.reader).unwrap();
    }
}

pub(super) fn receive_root_writer() -> Option<MacosProcessCapabilityChildWriter> {
    let socket = std::env::var_os(DESCRIPTOR_SOCKET_ENV)?;
    let stream = UnixStream::connect(socket).unwrap();
    stream.set_nonblocking(true).unwrap();
    let writer = receive_before(&stream);
    let marker = required_ready_path();
    std::fs::write(marker, []).unwrap();
    Some(writer)
}

pub(super) fn configure_descendant(
    writer: Option<&MacosProcessCapabilityChildWriter>,
    command: &mut Command,
) {
    if let Some(writer) = writer {
        command.env(DESCENDANT_DESCRIPTOR_ENV, writer.descriptor().to_string());
        writer.configure_exec_inheritance_for_test(command);
    }
}

pub(super) fn assert_descendant_writer() {
    let descriptor = std::env::var(DESCENDANT_DESCRIPTOR_ENV)
        .unwrap()
        .parse::<libc::c_int>()
        .unwrap();
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    assert_ne!(flags, FCNTL_FAILURE);
    assert_eq!(flags & libc::FD_CLOEXEC, libc::c_int::default());
}

fn accept_before(listener: &UnixListener) -> UnixStream {
    let deadline = Instant::now() + DESCRIPTOR_TIMEOUT;
    loop {
        match listener.accept() {
            Ok((stream, _address)) => return stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline, "{DESCRIPTOR_ACCEPT_DETAIL}");
                std::thread::sleep(DESCRIPTOR_POLL_INTERVAL);
            }
            Err(error) => panic!("{error}"),
        }
    }
}

fn receive_before(stream: &UnixStream) -> MacosProcessCapabilityChildWriter {
    let deadline = Instant::now() + DESCRIPTOR_TIMEOUT;
    loop {
        match crate::worker::receive_bootstrap_writer_once_for_test(stream.as_raw_fd()) {
            Ok(writer) => return writer,
            Err(error) if is_would_block(&error) => {
                assert!(Instant::now() < deadline, "{DESCRIPTOR_RECEIVE_DETAIL}");
                std::thread::sleep(DESCRIPTOR_POLL_INTERVAL);
            }
            Err(error) => panic!("{error:#}"),
        }
    }
}

fn wait_for_path(path: &Path, detail: &str) {
    let deadline = Instant::now() + DESCRIPTOR_TIMEOUT;
    while !path.exists() {
        assert!(Instant::now() < deadline, "{detail}");
        std::thread::sleep(DESCRIPTOR_POLL_INTERVAL);
    }
}

fn required_ready_path() -> PathBuf {
    PathBuf::from(std::env::var_os(super::process_fixture::TEST_HELPER_MARKER_ENV).unwrap())
        .with_extension(DESCRIPTOR_READY_EXTENSION)
}

fn is_would_block(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<std::io::Error>()
        .is_some_and(|error| error.kind() == std::io::ErrorKind::WouldBlock)
}

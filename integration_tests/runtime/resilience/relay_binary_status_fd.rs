use std::os::fd::{AsRawFd, RawFd};
use tokio::process::Command;
use yolo_runtime::cli::constants as cli;
use yolo_runtime::local_unix::LocalUnixStream;

const FCNTL_FAILURE: libc::c_int = -1;

pub(crate) struct InheritedStatusChannel {
    peer: LocalUnixStream,
    inherited: LocalUnixStream,
}

impl InheritedStatusChannel {
    pub(crate) fn prepare(command: &mut Command) -> Self {
        let (peer, inherited) = LocalUnixStream::pair().unwrap();
        let descriptor = inherited.as_raw_fd();
        assert!(descriptor > libc::STDERR_FILENO);
        command
            .arg(cli::STATUS_FD_ARGUMENT)
            .arg(descriptor.to_string());
        configure_inheritance(command, descriptor);
        Self { peer, inherited }
    }

    pub(crate) fn into_peer(self) -> LocalUnixStream {
        drop(self.inherited);
        self.peer
    }
}

fn configure_inheritance(command: &mut Command, descriptor: RawFd) {
    unsafe {
        command.pre_exec(move || {
            let flags = libc::fcntl(descriptor, libc::F_GETFD);
            if flags == FCNTL_FAILURE {
                return Err(std::io::Error::last_os_error());
            }
            let result = libc::fcntl(descriptor, libc::F_SETFD, flags & !libc::FD_CLOEXEC);
            if result == FCNTL_FAILURE {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

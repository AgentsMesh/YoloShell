use crate::process::sigchld_waitability::WaitableChildSpawnBoundary;
use crate::process::{LiveProcessIdentity, ProcessIdentity};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

const CHILD_EXECUTABLE: &[u8] = b"/bin/sleep\0";
const CHILD_LIFETIME: &[u8] = b"30\0";
const PIPE_ENDPOINT_COUNT: usize = 2;
const PIPE_READER_INDEX: usize = 0;
const PIPE_WRITER_INDEX: usize = 1;
const SYSCALL_SUCCESS: libc::c_int = 0;
const SYSCALL_FAILURE: libc::ssize_t = -1;
const HANDSHAKE_BYTE: u8 = b'!';
const HANDSHAKE_SIZE: libc::ssize_t = 1;
const EXEC_EOF_SIZE: libc::ssize_t = 0;

pub(super) struct ForkExecChild {
    pid: u32,
    identity: LiveProcessIdentity,
    release: Option<OwnedFd>,
    exec_observation: Option<OwnedFd>,
    reaped: bool,
}

impl ForkExecChild {
    pub(super) fn spawn(_boundary: WaitableChildSpawnBoundary<'_>) -> Self {
        let [ready_reader, ready_writer] = cloexec_pipe();
        let [release_reader, release_writer] = cloexec_pipe();
        let [exec_reader, exec_writer] = cloexec_pipe();
        // SAFETY: the child uses only async-signal-safe calls before exec.
        let pid = unsafe { libc::fork() };
        assert!(pid >= SYSCALL_SUCCESS);
        if pid == SYSCALL_SUCCESS {
            child_exec(
                ready_reader.as_raw_fd(),
                ready_writer.as_raw_fd(),
                release_reader.as_raw_fd(),
                release_writer.as_raw_fd(),
                exec_reader.as_raw_fd(),
                exec_writer.as_raw_fd(),
            );
        }
        drop(ready_writer);
        drop(release_reader);
        drop(exec_writer);
        assert_eq!(read_byte(ready_reader.as_raw_fd()), HANDSHAKE_SIZE);
        let pid = pid as u32;
        let durable = ProcessIdentity::capture(pid).unwrap();
        let identity = LiveProcessIdentity::capture(durable).unwrap().unwrap();
        Self {
            pid,
            identity,
            release: Some(release_writer),
            exec_observation: Some(exec_reader),
            reaped: false,
        }
    }

    pub(super) fn pid(&self) -> u32 {
        self.pid
    }

    pub(super) fn release_for_exec(&mut self) {
        let release = self.release.take().unwrap();
        assert_eq!(write_byte(release.as_raw_fd()), HANDSHAKE_SIZE);
    }

    pub(super) fn wait_for_exec(&mut self) {
        let observation = self.exec_observation.take().unwrap();
        assert_eq!(read_byte(observation.as_raw_fd()), EXEC_EOF_SIZE);
    }

    pub(super) fn assert_running(&mut self) {
        let mut status = libc::c_int::default();
        // SAFETY: this object exclusively reaps its direct child.
        let waited = unsafe { libc::waitpid(self.pid as libc::pid_t, &mut status, libc::WNOHANG) };
        self.reaped = waited == self.pid as libc::pid_t;
        assert_eq!(waited, SYSCALL_SUCCESS);
    }
}

impl Drop for ForkExecChild {
    fn drop(&mut self) {
        if self.reaped {
            return;
        }
        let _ = self.identity.signal(libc::SIGKILL);
        let mut status = libc::c_int::default();
        // SAFETY: waiting transfers the remaining child custody to this object.
        let _ =
            unsafe { libc::waitpid(self.pid as libc::pid_t, &mut status, libc::c_int::default()) };
        self.reaped = true;
    }
}

fn cloexec_pipe() -> [OwnedFd; PIPE_ENDPOINT_COUNT] {
    let mut raw = [libc::c_int::default(); PIPE_ENDPOINT_COUNT];
    // SAFETY: raw has storage for exactly two descriptors.
    assert_eq!(unsafe { libc::pipe(raw.as_mut_ptr()) }, SYSCALL_SUCCESS);
    // SAFETY: each successful pipe descriptor is transferred exactly once.
    let descriptors = unsafe {
        [
            OwnedFd::from_raw_fd(raw[PIPE_READER_INDEX]),
            OwnedFd::from_raw_fd(raw[PIPE_WRITER_INDEX]),
        ]
    };
    for descriptor in &descriptors {
        crate::process::macos_process_capability_fd::set_close_on_exec(
            descriptor.as_raw_fd(),
            true,
        )
        .unwrap();
    }
    descriptors
}

fn read_byte(descriptor: libc::c_int) -> libc::ssize_t {
    let mut byte = u8::default();
    // SAFETY: byte remains writable for the duration of the retry loop.
    unsafe { read_one(descriptor, std::ptr::addr_of_mut!(byte)) }
}

fn write_byte(descriptor: libc::c_int) -> libc::ssize_t {
    let byte = HANDSHAKE_BYTE;
    // SAFETY: byte remains readable for the duration of the retry loop.
    unsafe { write_one(descriptor, std::ptr::addr_of!(byte)) }
}

unsafe fn read_one(descriptor: libc::c_int, byte: *mut u8) -> libc::ssize_t {
    loop {
        let read = unsafe { libc::read(descriptor, byte.cast(), std::mem::size_of::<u8>()) };
        if read != SYSCALL_FAILURE || !was_interrupted() {
            return read;
        }
    }
}

unsafe fn write_one(descriptor: libc::c_int, byte: *const u8) -> libc::ssize_t {
    loop {
        let written = unsafe { libc::write(descriptor, byte.cast(), std::mem::size_of::<u8>()) };
        if written != SYSCALL_FAILURE || !was_interrupted() {
            return written;
        }
    }
}

fn was_interrupted() -> bool {
    // SAFETY: __error returns the calling thread's errno storage on macOS.
    unsafe { *libc::__error() == libc::EINTR }
}

fn child_exec(
    ready_read: i32,
    ready_write: i32,
    release_read: i32,
    release_write: i32,
    exec_read: i32,
    _exec_write: i32,
) -> ! {
    unsafe {
        libc::close(ready_read);
        libc::close(release_write);
        libc::close(exec_read);
        let byte = HANDSHAKE_BYTE;
        let ready = write_one(ready_write, std::ptr::addr_of!(byte));
        let mut release = 0_u8;
        let released = read_one(release_read, std::ptr::addr_of_mut!(release));
        if ready == HANDSHAKE_SIZE && released == HANDSHAKE_SIZE {
            libc::close(ready_write);
            libc::close(release_read);
            // The exec-observation writer intentionally closes only through FD_CLOEXEC.
            let executable = CHILD_EXECUTABLE.as_ptr().cast::<libc::c_char>();
            let lifetime = CHILD_LIFETIME.as_ptr().cast::<libc::c_char>();
            libc::execl(
                executable,
                executable,
                lifetime,
                std::ptr::null::<libc::c_char>(),
            );
        }
        libc::_exit(libc::EXIT_FAILURE)
    }
}

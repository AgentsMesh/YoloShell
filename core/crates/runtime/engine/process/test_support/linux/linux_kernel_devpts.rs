use std::fs::File;
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

pub(super) const WRITER_READY: &[u8] = b"ready";
const READY_TIMEOUT: std::time::Duration = crate::session_timing::WORKER_READY_TIMEOUT;
const READY_DETAIL: &str = "stale writer did not close its inherited PTY descriptors";
const POLL_DESCRIPTOR_COUNT: libc::nfds_t = 1;
const POLL_READY_COUNT: libc::c_int = 1;

pub(super) fn channel() -> [File; 2] {
    let (child, parent) = std::os::unix::net::UnixStream::pair().expect(READY_DETAIL);
    [child, parent].map(|stream| File::from(OwnedFd::from(stream)))
}

pub(super) fn wait_ready(stream: &mut File) {
    let deadline = std::time::Instant::now() + READY_TIMEOUT;
    let mut state = libc::pollfd {
        fd: stream.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let mut received = [u8::default(); WRITER_READY.len()];
    let mut offset = usize::default();
    while offset < received.len() {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        assert!(!remaining.is_zero(), "{READY_DETAIL}");
        let result = unsafe {
            libc::poll(
                &mut state,
                POLL_DESCRIPTOR_COUNT,
                remaining.as_millis().try_into().unwrap(),
            )
        };
        if result < libc::c_int::default()
            && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted
        {
            continue;
        }
        assert_eq!(result, POLL_READY_COUNT, "{READY_DETAIL}");
        let count = stream.read(&mut received[offset..]).expect(READY_DETAIL);
        assert!(count > usize::default(), "{READY_DETAIL}");
        offset += count;
    }
    assert_eq!(&received, WRITER_READY, "{READY_DETAIL}");
}

pub(super) fn adopt(descriptor: RawFd) -> File {
    let file = unsafe { File::from_raw_fd(descriptor) };
    let descriptor = file.as_raw_fd();
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    assert!(flags >= 0, "inspect fixture PTY inheritance");
    assert_eq!(
        unsafe { libc::fcntl(descriptor, libc::F_SETFD, flags | libc::FD_CLOEXEC) },
        libc::c_int::default(),
        "fixture PTY must not survive an unrelated exec"
    );
    file
}

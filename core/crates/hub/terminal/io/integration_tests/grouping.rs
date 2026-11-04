use super::constants::*;
use std::os::fd::AsRawFd;
use yolo_hub_terminal_io::Descriptor;

#[tokio::test]
async fn aliased_descriptors_remain_nonblocking_until_the_entire_group_is_retired() {
    for drop_first in [false, true] {
        let (socket, _peer) = std::os::unix::net::UnixStream::pair().unwrap();
        let source = socket.as_raw_fd();
        let original = unsafe { libc::fcntl(source, libc::F_GETFL) };
        let (first, second) = Descriptor::pair(source, source).unwrap();
        let remaining = if drop_first {
            drop(first);
            second
        } else {
            drop(second);
            first
        };
        assert_ne!(
            unsafe { libc::fcntl(source, libc::F_GETFL) } & libc::O_NONBLOCK,
            0
        );
        drop(remaining);
        assert_eq!(unsafe { libc::fcntl(source, libc::F_GETFL) }, original);
    }
}

#[tokio::test]
async fn mixed_original_flags_are_restored_and_capture_failure_changes_nothing() {
    let (first, second) = std::os::unix::net::UnixStream::pair().unwrap();
    first.set_nonblocking(true).unwrap();
    let original_first = unsafe { libc::fcntl(first.as_raw_fd(), libc::F_GETFL) };
    let original_second = unsafe { libc::fcntl(second.as_raw_fd(), libc::F_GETFL) };
    let (one, two, three) =
        Descriptor::triple(first.as_raw_fd(), second.as_raw_fd(), second.as_raw_fd()).unwrap();
    drop(two);
    drop(one);
    assert_ne!(
        unsafe { libc::fcntl(second.as_raw_fd(), libc::F_GETFL) } & libc::O_NONBLOCK,
        0
    );
    drop(three);
    assert_eq!(
        unsafe { libc::fcntl(first.as_raw_fd(), libc::F_GETFL) },
        original_first
    );
    assert_eq!(
        unsafe { libc::fcntl(second.as_raw_fd(), libc::F_GETFL) },
        original_second
    );
    assert!(Descriptor::pair(second.as_raw_fd(), BAD_DESCRIPTOR).is_err());
    assert_eq!(
        unsafe { libc::fcntl(second.as_raw_fd(), libc::F_GETFL) },
        original_second
    );
}

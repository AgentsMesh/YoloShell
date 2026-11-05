use super::*;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

const TEST_COLUMNS: u16 = 89;
const TEST_ROWS: u16 = 47;
const TEST_CHANNEL_CAPACITY: usize = 1;
const INVALID_DESCRIPTOR: RawFd = -1;
const SYSCALL_SUCCESS: libc::c_int = 0;
const NO_PIXEL_DIMENSION: u16 = 0;

#[tokio::test]
async fn current_size_publisher_ignores_non_terminals_and_descriptor_failures() {
    let file = tempfile::tempfile().unwrap();
    let (sender, mut receiver) = mpsc::channel(TEST_CHANNEL_CAPACITY);

    assert!(publish_current_size(file.as_raw_fd(), &sender).await);
    assert!(publish_current_size(INVALID_DESCRIPTOR, &sender).await);
    assert!(receiver.try_recv().is_err());
}

#[tokio::test]
async fn current_size_publisher_stops_after_its_receiver_is_closed() {
    let (_master, slave) = open_pty();
    let (sender, receiver) = mpsc::channel(TEST_CHANNEL_CAPACITY);
    drop(receiver);

    assert!(!publish_current_size(slave.as_raw_fd(), &sender).await);
}

fn open_pty() -> (OwnedFd, OwnedFd) {
    let mut master = INVALID_DESCRIPTOR;
    let mut slave = INVALID_DESCRIPTOR;
    let mut size = libc::winsize {
        ws_row: TEST_ROWS,
        ws_col: TEST_COLUMNS,
        ws_xpixel: NO_PIXEL_DIMENSION,
        ws_ypixel: NO_PIXEL_DIMENSION,
    };
    // SAFETY: openpty initializes both descriptors and retains no supplied pointers.
    let result = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut size,
        )
    };
    assert_eq!(result, SYSCALL_SUCCESS);
    // SAFETY: successful openpty returned two newly owned descriptors.
    unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) }
}

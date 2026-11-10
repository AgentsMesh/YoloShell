use super::constants::*;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use yolo_hub_terminal_io::{RawTerminal, terminal_size};

#[test]
fn raw_terminal_restores_attributes_using_its_own_retained_descriptor() {
    let mut master = BAD_DESCRIPTOR;
    let mut slave = BAD_DESCRIPTOR;
    let mut size = libc::winsize {
        ws_col: COLUMNS,
        ws_row: ROWS,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    assert_eq!(
        unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut size,
            )
        },
        SYSCALL_SUCCESS
    );
    let master = unsafe { OwnedFd::from_raw_fd(master) };
    let slave = unsafe { OwnedFd::from_raw_fd(slave) };
    let before = attributes(slave.as_raw_fd());
    let raw = RawTerminal::enable(slave.as_raw_fd()).unwrap().unwrap();
    let changed = attributes(slave.as_raw_fd());
    assert_eq!(
        changed.c_lflag & (libc::ICANON | libc::ECHO | libc::ISIG),
        0
    );
    let size = terminal_size(slave.as_raw_fd()).unwrap().unwrap();
    assert_eq!(size.cols(), COLUMNS);
    assert_eq!(size.rows(), ROWS);
    drop(raw);
    let restored = attributes(slave.as_raw_fd());
    // PENDIN is kernel-owned bookkeeping for input pending canonical reprocessing.
    assert_eq!(
        restored.c_lflag & CONFIGURATION_LFLAGS,
        before.c_lflag & CONFIGURATION_LFLAGS
    );
    assert_eq!(restored.c_iflag, before.c_iflag);
    assert_eq!(restored.c_oflag, before.c_oflag);
    assert_eq!(restored.c_cflag, before.c_cflag);
    assert_eq!(restored.c_cc, before.c_cc);
    drop(master);
}

#[test]
fn non_terminal_and_invalid_descriptors_are_not_treated_as_default_terminal_geometry() {
    let (socket, _peer) = std::os::unix::net::UnixStream::pair().unwrap();
    assert!(RawTerminal::enable(socket.as_raw_fd()).unwrap().is_none());
    assert!(terminal_size(socket.as_raw_fd()).unwrap().is_none());
    assert!(RawTerminal::enable(BAD_DESCRIPTOR).is_err());
    assert!(terminal_size(BAD_DESCRIPTOR).is_err());
}

fn attributes(fd: libc::c_int) -> libc::termios {
    let mut termios = std::mem::MaybeUninit::<libc::termios>::uninit();
    assert_eq!(
        unsafe { libc::tcgetattr(fd, termios.as_mut_ptr()) },
        SYSCALL_SUCCESS
    );
    unsafe { termios.assume_init() }
}

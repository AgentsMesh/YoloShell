use super::*;
use crate::model::{MIN_TERMINAL_COLUMNS, MIN_TERMINAL_ROWS};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::ptr;
use std::time::Duration;

const WINDOW_CHANGE_TIMEOUT: Duration = Duration::from_secs(1);

fn open_pty(size: TerminalSize) -> (OwnedFd, OwnedFd) {
    let mut master = -1;
    let mut slave = -1;
    let mut window = libc::winsize {
        ws_row: size.rows,
        ws_col: size.cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: openpty initializes both descriptors and does not retain the supplied pointers.
    let result = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut window,
        )
    };
    assert_eq!(result, 0);
    // SAFETY: openpty returned two newly owned descriptors.
    unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) }
}

fn terminal_flags(fd: i32) -> libc::tcflag_t {
    let mut value = std::mem::MaybeUninit::<libc::termios>::uninit();
    // SAFETY: the test supplies an open PTY descriptor and tcgetattr initializes value.
    assert_eq!(unsafe { libc::tcgetattr(fd, value.as_mut_ptr()) }, 0);
    // SAFETY: tcgetattr succeeded.
    unsafe { value.assume_init() }.c_lflag
}

#[test]
fn options_require_a_nonempty_holder_and_terminal_size_rejects_zero() {
    let id = Uuid::new_v4();
    assert!(matches!(
        RelayOptions::new(id, "  "),
        Err(RelayError::InvalidHolderId)
    ));
    let options = RelayOptions::new(id, "holder-a")
        .unwrap()
        .after_sequence(12)
        .initial_size(TerminalSize::new(90, 30))
        .take_control(true);
    assert_eq!(options.holder_id, "holder-a");
    assert!(options.take_control);
    assert_eq!(options.after_sequence, 12);
    assert_eq!(
        options.initial_size,
        Some(TerminalSize { cols: 90, rows: 30 })
    );
    assert_eq!(TerminalSize::new(0, 30), None);
    assert_eq!(TerminalSize::new(90, 0), None);
    assert_eq!(
        TerminalSize::new(MIN_TERMINAL_ROWS, MIN_TERMINAL_ROWS),
        Some(TerminalSize {
            cols: MIN_TERMINAL_COLUMNS,
            rows: MIN_TERMINAL_ROWS,
        })
    );
}

#[test]
fn raw_terminal_restores_flags_and_reads_window_size() {
    const RESTORED_FLAGS: libc::tcflag_t = libc::ICANON | libc::ECHO | libc::ISIG | libc::IEXTEN;
    let (_master, slave) = open_pty(TerminalSize { cols: 91, rows: 37 });
    let fd = slave.as_raw_fd();
    let original = terminal_flags(fd);
    assert_eq!(terminal_size(fd).unwrap(), TerminalSize::new(91, 37));
    let guard = RawTerminal::enable(fd).unwrap().unwrap();
    assert_eq!(terminal_flags(fd) & libc::ICANON, 0);
    assert_eq!(terminal_flags(fd) & libc::ECHO, 0);
    drop(guard);
    assert_eq!(
        terminal_flags(fd) & RESTORED_FLAGS,
        original & RESTORED_FLAGS
    );
}

#[test]
fn regular_files_are_not_treated_as_terminals() {
    let file = tempfile::tempfile().unwrap();
    assert_eq!(terminal_size(file.as_raw_fd()).unwrap(), None);
    assert!(RawTerminal::enable(file.as_raw_fd()).unwrap().is_none());
    assert!(terminal_size(-1).is_err());
    assert!(RawTerminal::enable(-1).is_err());
}

#[tokio::test]
async fn window_change_watcher_publishes_the_current_size() {
    let (_master, slave) = open_pty(TerminalSize { cols: 77, rows: 29 });
    let mut changes = watch_window_changes(slave.as_raw_fd()).unwrap();
    // SAFETY: raising SIGWINCH in this process is supported and Tokio installed its handler.
    assert_eq!(unsafe { libc::raise(libc::SIGWINCH) }, 0);
    let size = tokio::time::timeout(WINDOW_CHANGE_TIMEOUT, changes.recv())
        .await
        .unwrap();
    assert_eq!(size, TerminalSize::new(77, 29));
    drop(changes);
    // Exercise the watcher shutdown path after its receiver disappears.
    assert_eq!(unsafe { libc::raise(libc::SIGWINCH) }, 0);
}

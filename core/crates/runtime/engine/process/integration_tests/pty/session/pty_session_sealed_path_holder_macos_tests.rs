use super::super::constants::MAX_PTY_PROOF_CANDIDATES;
use super::macos_path_holders::path_holders;
use portable_pty::{PtySize, native_pty_system};
use std::fs::OpenOptions;
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;
const SEALED_MODE: libc::mode_t = 0;
const SYSCALL_FAILURE: libc::c_int = -1;

#[test]
fn sealed_slave_path_remains_queryable_after_other_slave_closure() {
    let pair = native_pty_system().openpty(pty_size()).unwrap();
    let path = pair.master.tty_name().unwrap();
    drop(pair.slave);
    let retained = open_slave(&path);
    let transient = open_slave(&path);
    assert_ne!(
        unsafe { libc::fchmod(retained.as_raw_fd(), SEALED_MODE) },
        SYSCALL_FAILURE
    );

    assert!(
        path_holders(&path, MAX_PTY_PROOF_CANDIDATES)
            .unwrap()
            .contains(&std::process::id())
    );
    drop(transient);
    assert!(
        path_holders(&path, MAX_PTY_PROOF_CANDIDATES)
            .unwrap()
            .contains(&std::process::id())
    );
}

fn open_slave(path: &Path) -> std::fs::File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)
        .unwrap()
}

fn pty_size() -> PtySize {
    PtySize {
        rows: TEST_ROWS,
        cols: TEST_COLUMNS,
        pixel_width: TEST_PIXEL_SIZE,
        pixel_height: TEST_PIXEL_SIZE,
    }
}

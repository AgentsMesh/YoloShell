use std::fs::{File, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;

const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
pub(super) const PAYLOAD: &[u8] = b"accepted-pty-output";
pub(super) const EMPTY_READ: usize = 1;
pub(super) const TEST_ROUNDS: usize = 16;

pub(super) struct Fixture {
    pub master: File,
    pub slave: File,
    pub path: PathBuf,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let pair = portable_pty::native_pty_system()
            .openpty(portable_pty::PtySize {
                rows: TEST_ROWS,
                cols: TEST_COLUMNS,
                pixel_width: u16::default(),
                pixel_height: u16::default(),
            })
            .unwrap();
        let path = pair.master.tty_name().unwrap();
        let slave = open_slave(&path);
        let super::super::OutputDescriptor::Native(master) =
            super::super::capture(pair.master.as_ref()).unwrap()
        else {
            panic!("native PTY fixture must retain a master descriptor");
        };
        Self {
            master,
            slave,
            path,
        }
    }
}

pub(super) fn open_slave(path: &std::path::Path) -> File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY | libc::O_CLOEXEC)
        .open(path)
        .unwrap()
}

use crate::worker::tests::*;
#[cfg(any(target_os = "linux", target_os = "macos"))]
const NATIVE_PTY_TEST_ROWS: u16 = 2;
#[cfg(any(target_os = "linux", target_os = "macos"))]
const NATIVE_PTY_TEST_COLS: u16 = 4;
#[cfg(any(target_os = "linux", target_os = "macos"))]
const NATIVE_PTY_TEST_PIXEL_SIZE: u16 = 0;
#[cfg(any(target_os = "linux", target_os = "macos"))]
const NATIVE_PTY_READ_BYTES: usize = 8;
#[cfg(any(target_os = "linux", target_os = "macos"))]
const NATIVE_PTY_EMPTY_BYTE: u8 = 0;
#[cfg(any(target_os = "linux", target_os = "macos"))]
const NATIVE_PTY_EOF_BYTES: usize = 0;
#[cfg(any(target_os = "linux", target_os = "macos"))]
const NATIVE_PTY_UNEXPECTED_OUTPUT_DETAIL: &str = "closed native PTY returned unexpected output";

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn supported_native_targets_recognize_eio_as_a_kernel_pty_eof() {
    let error = io::Error::from_raw_os_error(libc::EIO);
    assert!(pty_slave_closed(&error));
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
#[test]
fn unsupported_target_does_not_treat_eio_as_kernel_pty_eof() {
    let error = io::Error::from_raw_os_error(libc::EIO);
    assert!(!pty_slave_closed(&error));
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn native_pty_slave_close_reports_a_kernel_end_condition() {
    let system = native_pty_system();
    let pair = system
        .openpty(PtySize {
            rows: NATIVE_PTY_TEST_ROWS,
            cols: NATIVE_PTY_TEST_COLS,
            pixel_width: NATIVE_PTY_TEST_PIXEL_SIZE,
            pixel_height: NATIVE_PTY_TEST_PIXEL_SIZE,
        })
        .unwrap();
    let fd = configure_nonblocking_pty(pair.master.as_ref()).unwrap();
    let mut reader = pair.master.try_clone_reader().unwrap();
    drop(pair.slave);

    assert!(matches!(
        wait_for_pty_readiness(fd, &AtomicBool::new(false), &AtomicBool::new(false),).unwrap(),
        PtyReadiness::Readable | PtyReadiness::Closed
    ));
    let mut buffer = [NATIVE_PTY_EMPTY_BYTE; NATIVE_PTY_READ_BYTES];
    match reader.read(&mut buffer) {
        Ok(NATIVE_PTY_EOF_BYTES) => {}
        Err(error) => {
            assert_eq!(error.raw_os_error(), Some(libc::EIO));
            assert!(pty_slave_closed(&error));
        }
        Ok(_) => panic!("{NATIVE_PTY_UNEXPECTED_OUTPUT_DETAIL}"),
    }
}

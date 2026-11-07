use super::*;
use portable_pty::{PtySize, native_pty_system};
use std::os::unix::fs::MetadataExt;

const TEST_COLUMNS: u16 = 80;
const TEST_ROWS: u16 = 24;
const TEST_PIXELS: u16 = 0;
const STALE_DEVICE_OFFSET: u64 = 1;

#[test]
fn duplicate_and_transfer_preserve_the_exact_live_slave() {
    let pair = pair();
    let device = std::fs::metadata(pair.master.tty_name().unwrap())
        .unwrap()
        .rdev();
    let source = pair.master.as_raw_fd().unwrap();
    let duplicate = LinuxPassivePtyMaster::duplicate_exact(source, device).unwrap();
    let expected = duplicate.slave_identity();
    let transferred =
        LinuxPassivePtyMaster::from_transferred(duplicate.into_descriptor(), device).unwrap();

    assert_eq!(transferred.slave_identity(), expected);
    assert_ne!(transferred.descriptor().as_raw_fd(), source);
}

#[test]
fn another_slave_device_is_rejected() {
    let pair = pair();
    let device = std::fs::metadata(pair.master.tty_name().unwrap())
        .unwrap()
        .rdev();
    LinuxPassivePtyMaster::duplicate_exact(
        pair.master.as_raw_fd().unwrap(),
        device + STALE_DEVICE_OFFSET,
    )
    .err()
    .expect("mismatched PTY device was accepted");
}

#[test]
fn a_non_master_descriptor_is_rejected() {
    let descriptors = pipe();
    LinuxPassivePtyMaster::from_transferred(descriptors[0].try_clone().unwrap().into(), 1)
        .err()
        .expect("pipe descriptor was accepted as a PTY master");
}

fn pair() -> portable_pty::PtyPair {
    native_pty_system()
        .openpty(PtySize {
            rows: TEST_ROWS,
            cols: TEST_COLUMNS,
            pixel_width: TEST_PIXELS,
            pixel_height: TEST_PIXELS,
        })
        .unwrap()
}

fn pipe() -> [std::fs::File; 2] {
    let mut descriptors = [libc::c_int::default(); 2];
    assert_eq!(unsafe { libc::pipe(descriptors.as_mut_ptr()) }, 0);
    descriptors.map(|descriptor| unsafe { std::fs::File::from_raw_fd(descriptor) })
}

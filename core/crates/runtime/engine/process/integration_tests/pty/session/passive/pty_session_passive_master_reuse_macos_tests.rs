use super::*;
use portable_pty::{PtySize, native_pty_system};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::path::Path;

const TEST_CAPABILITY: &str = "passive-master-reuse-capability";
const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;
const REUSE_SEARCH_LIMIT: usize = 256;
const MINIMUM_TEST_DESCRIPTOR: libc::c_int = libc::STDERR_FILENO + 1;
const FCNTL_FAILURE: libc::c_int = -1;
const REUSED_MASTER_ACCEPTED_DETAIL: &str = "reused PTY path authorized a new PTY object";
const REUSE_NOT_OBSERVED_DETAIL: &str = "released PTY path was not reused within the test bound";

#[test]
fn reused_path_with_a_new_pty_object_is_rejected() {
    let original = open_pty();
    let path = original.master.tty_name().unwrap();
    let proof = PtySessionProof::capture(path.clone(), TEST_CAPABILITY.to_owned()).unwrap();
    let original_identity = proof.pty_object_identity().unwrap();
    drop(original);
    let replacement = find_reused(&path);
    let replacement_identity =
        MacosPtyObjectIdentity::from_metadata(&std::fs::metadata(&path).unwrap());
    assert_ne!(replacement_identity, original_identity);
    let descriptor = duplicate(replacement.master.as_raw_fd().unwrap());
    let raw = descriptor.as_raw_fd();

    MacosPassivePtyMaster::from_transferred_exact(descriptor, &proof)
        .err()
        .expect(REUSED_MASTER_ACCEPTED_DETAIL);

    assert_closed(raw);
}

#[test]
fn passive_alias_is_the_only_extra_pty_custody_and_releases_on_drop() {
    let pair = open_pty();
    let path = pair.master.tty_name().unwrap();
    let proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned()).unwrap();
    let passive = MacosPassivePtyMaster::duplicate_exact(&proof).unwrap();
    drop(pair);
    drop(proof);

    let while_retained = open_pty();
    assert_ne!(while_retained.master.tty_name().unwrap(), path);
    drop(while_retained);

    drop(passive);
    let replacement = find_reused(&path);
    assert_eq!(replacement.master.tty_name().unwrap(), path);
}

fn find_reused(expected: &Path) -> portable_pty::PtyPair {
    let mut retained = Vec::new();
    for _attempt in 0..REUSE_SEARCH_LIMIT {
        let candidate = open_pty();
        if candidate.master.tty_name().unwrap() == expected {
            return candidate;
        }
        retained.push(candidate);
    }
    panic!("{REUSE_NOT_OBSERVED_DETAIL}");
}

fn open_pty() -> portable_pty::PtyPair {
    native_pty_system()
        .openpty(PtySize {
            rows: TEST_ROWS,
            cols: TEST_COLUMNS,
            pixel_width: TEST_PIXEL_SIZE,
            pixel_height: TEST_PIXEL_SIZE,
        })
        .unwrap()
}

fn duplicate(source: libc::c_int) -> OwnedFd {
    let raw = unsafe { libc::fcntl(source, libc::F_DUPFD, MINIMUM_TEST_DESCRIPTOR) };
    assert_ne!(raw, FCNTL_FAILURE);
    unsafe { OwnedFd::from_raw_fd(raw) }
}

fn assert_closed(descriptor: libc::c_int) {
    assert_eq!(
        unsafe { libc::fcntl(descriptor, libc::F_GETFD) },
        FCNTL_FAILURE
    );
}

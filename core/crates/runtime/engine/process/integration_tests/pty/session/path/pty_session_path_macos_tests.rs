use super::*;
use portable_pty::{PtySize, native_pty_system};

const EXPECTED_FILESYSTEM_DEVICE: u32 = 41;
const EXPECTED_INODE: u64 = 42;
const REPLACEMENT_INODE: u64 = 43;
const EXPECTED_TERMINAL_RDEV: u32 = 44;
const REPLACEMENT_TERMINAL_RDEV: u32 = 45;
const TEST_ALIAS_COMPONENT: &str = "tty-alias";
const TEST_CAPABILITY: &str = "path-identity-capability";
const TEST_DEVICE_PATH: &str = "/dev/null";
const REUSE_SEARCH_LIMIT: usize = 256;
const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;
const REUSE_NOT_OBSERVED_DETAIL: &str = "captured PTY path was not reused within the test bound";

#[test]
fn proof_capture_retains_the_canonical_character_device_path() {
    let directory = tempfile::tempdir().unwrap();
    let alias = directory.path().join(TEST_ALIAS_COMPONENT);
    std::os::unix::fs::symlink(TEST_DEVICE_PATH, &alias).unwrap();

    let proof = PtySessionProof::capture(alias, TEST_CAPABILITY.to_owned()).unwrap();

    assert_eq!(
        proof.tty_path_state().unwrap(),
        TtyPathState::Available(proof.pty_object_identity().unwrap())
    );
    assert_eq!(
        proof.tty_path,
        std::fs::canonicalize(TEST_DEVICE_PATH).unwrap()
    );
}

#[test]
fn missing_captured_path_selects_released_path_scanning() {
    let mut proof = PtySessionProof::capture(
        std::path::PathBuf::from(TEST_DEVICE_PATH),
        TEST_CAPABILITY.to_owned(),
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    proof.tty_path = directory.path().join(TEST_ALIAS_COMPONENT);

    assert_eq!(proof.tty_path_state().unwrap(), TtyPathState::Released);
}

#[test]
fn tty_identity_requires_a_character_device() {
    let error = validate_tty_identity(
        expected(),
        TtyPathIdentity {
            is_character_device: false,
            object: expected(),
        },
    )
    .unwrap_err();

    assert_eq!(error.to_string(), TTY_TYPE_DETAIL);
}

#[test]
fn tty_identity_rejects_terminal_device_replacement() {
    let error = validate_tty_identity(
        expected(),
        TtyPathIdentity {
            is_character_device: true,
            object: identity(EXPECTED_INODE, REPLACEMENT_TERMINAL_RDEV),
        },
    )
    .unwrap_err();

    assert_eq!(error.to_string(), TTY_IDENTITY_DETAIL);
}

#[test]
fn tty_identity_rejects_same_rdev_with_a_replaced_inode() {
    let error = validate_tty_identity(
        expected(),
        TtyPathIdentity {
            is_character_device: true,
            object: identity(REPLACEMENT_INODE, EXPECTED_TERMINAL_RDEV),
        },
    )
    .unwrap_err();

    assert_eq!(error.to_string(), TTY_IDENTITY_DETAIL);
}

#[test]
fn tty_identity_accepts_the_captured_character_device() {
    validate_tty_identity(
        expected(),
        TtyPathIdentity {
            is_character_device: true,
            object: expected(),
        },
    )
    .unwrap();
}

#[test]
fn real_pty_path_reuse_with_a_new_inode_is_rejected() {
    let original = test_pty();
    let path = original.master.tty_name().unwrap();
    let proof = PtySessionProof::capture(path.clone(), TEST_CAPABILITY.to_owned()).unwrap();
    let captured = proof.pty_object_identity().unwrap();
    drop(original);
    let mut retained = Vec::new();

    for _attempt in 0..REUSE_SEARCH_LIMIT {
        let replacement = test_pty();
        if replacement.master.tty_name().unwrap() == path {
            let metadata = std::fs::metadata(&path).unwrap();
            let reused = MacosPtyObjectIdentity::from_metadata(&metadata);
            assert_eq!(captured.filesystem_device(), reused.filesystem_device());
            assert_eq!(captured.terminal_rdev(), reused.terminal_rdev());
            assert_ne!(captured.inode(), reused.inode());
            assert!(proof.tty_path_state().is_err());
            return;
        }
        retained.push(replacement);
    }
    panic!("{REUSE_NOT_OBSERVED_DETAIL}");
}

fn expected() -> MacosPtyObjectIdentity {
    identity(EXPECTED_INODE, EXPECTED_TERMINAL_RDEV)
}

fn identity(inode: u64, terminal_rdev: u32) -> MacosPtyObjectIdentity {
    MacosPtyObjectIdentity::from_kernel_fields(EXPECTED_FILESYSTEM_DEVICE, inode, terminal_rdev)
}

fn test_pty() -> portable_pty::PtyPair {
    native_pty_system()
        .openpty(PtySize {
            rows: TEST_ROWS,
            cols: TEST_COLUMNS,
            pixel_width: TEST_PIXEL_SIZE,
            pixel_height: TEST_PIXEL_SIZE,
        })
        .unwrap()
}

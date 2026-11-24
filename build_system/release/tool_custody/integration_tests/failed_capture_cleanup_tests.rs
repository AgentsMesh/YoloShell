use crate::TrustedBazel;
use std::ffi::OsString;
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use yolo_release_contracts::{PINNED_BAZEL_VERSION, PRIVATE_EXECUTABLE_MODE, PRIVATE_FILE_MODE};
use yolo_release_foundation::digest::bytes_sha256;

const SOURCE_NAME: &str = "version-fixture";
const PROBE_NAME: &str = "execution-path";
const PROBE_SCRIPT: &str = "#!/bin/bash\nprintf '%s' \"$0\" > \"$1\"\n";
const BAD_VERSION: &str = "printf 'unexpected\\n'\n";
const FAILED_VERSION: &str = "exit 1\n";
const NON_UTF8_VERSION: &str = "printf '\\377\\n'\n";
const INVALID_UTF8: u8 = 0xff;

fn executable(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_EXECUTABLE_MODE)).unwrap();
}

fn assert_private_copy_removed(probe: &Path, source: &Path) {
    let copied = PathBuf::from(fs::read_to_string(probe).unwrap());
    assert_ne!(copied, source);
    assert!(!copied.exists());
    assert!(!copied.parent().unwrap().exists());
    assert!(source.exists());
}

#[test]
fn failed_version_validation_removes_the_sealed_private_tool_copy() {
    for response in [BAD_VERSION, FAILED_VERSION, NON_UTF8_VERSION] {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join(SOURCE_NAME);
        let probe = root.path().join(PROBE_NAME);
        let script = format!("{PROBE_SCRIPT}{response}");
        executable(&source, script.as_bytes());
        assert!(
            TrustedBazel::validate_identity_with_argument_for_test(
                &source,
                &bytes_sha256(script.as_bytes()),
                probe.to_str().unwrap(),
                PINNED_BAZEL_VERSION,
            )
            .is_err()
        );
        assert_private_copy_removed(&probe, &source);
    }
}

#[test]
fn non_utf8_missing_source_is_rejected_before_a_private_copy_is_created() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join(OsString::from_vec(vec![INVALID_UTF8]));
    let probe = root.path().join(PROBE_NAME);
    let script = format!("{PROBE_SCRIPT}printf '%s\\n' '{PINNED_BAZEL_VERSION}'\n");
    assert!(
        TrustedBazel::validate_identity_with_argument_for_test(
            &source,
            &bytes_sha256(script.as_bytes()),
            probe.to_str().unwrap(),
            PINNED_BAZEL_VERSION,
        )
        .is_err()
    );
    assert!(!probe.exists());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), usize::default());
}

#[test]
fn missing_directory_and_nonexecutable_inputs_fail_before_private_copying() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join(SOURCE_NAME);
    let digest = bytes_sha256(PROBE_SCRIPT.as_bytes());
    assert!(
        TrustedBazel::validate_identity_for_test(&source, &digest, PINNED_BAZEL_VERSION).is_err()
    );
    assert!(
        TrustedBazel::validate_identity_for_test(root.path(), &digest, PINNED_BAZEL_VERSION)
            .is_err()
    );
    fs::write(&source, PROBE_SCRIPT).unwrap();
    fs::set_permissions(&source, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    assert!(
        TrustedBazel::validate_identity_for_test(&source, &digest, PINNED_BAZEL_VERSION).is_err()
    );
    assert_eq!(fs::read(&source).unwrap(), PROBE_SCRIPT.as_bytes());
}

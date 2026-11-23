use super::{LinuxBazelTool, LinuxBazelToolchains, PinnedLinuxBazel};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use yolo_release_foundation::digest::bytes_sha256;
use yolo_release_foundation::validation::SHA256_HEX_LENGTH;
use yolo_release_gate_system::{LINUX_AMD64_PROFILE, LINUX_ARM64_PROFILE};

const EXECUTABLE_MODE: u32 = 0o700;
const PINNED_BYTES: &[u8] = b"pinned Linux Bazel cleanup witness";
const PRIVATE_NAME: &str = "bazel-linux";
const MOVED_NAME: &str = "retained-original";
const FOREIGN_NAME: &str = "foreign-sentinel";
const FOREIGN_BYTES: &[u8] = b"unrelated replacement";
const READ_ONLY_MODE: u32 = 0o400;
const MODE_MASK: u32 = 0o777;
const AMD64_NAME: &str = "bazel-amd64";
const ARM64_NAME: &str = "bazel-arm64";
const AMD64_BYTES: &[u8] = b"pinned amd64 Bazel fixture";
const ARM64_BYTES: &[u8] = b"pinned arm64 Bazel fixture";

#[test]
fn architecture_profiles_select_their_own_retained_bazel_image() {
    let root = tempfile::tempdir().unwrap();
    let capture = |name: &str, bytes: &[u8]| {
        let path = root.path().join(name);
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(EXECUTABLE_MODE)).unwrap();
        PinnedLinuxBazel::capture(&path, &bytes_sha256(bytes)).unwrap()
    };
    let toolchains = LinuxBazelToolchains {
        amd64: capture(AMD64_NAME, AMD64_BYTES),
        arm64: capture(ARM64_NAME, ARM64_BYTES),
    };

    for (profile, expected) in [
        (LINUX_AMD64_PROFILE, AMD64_BYTES),
        (LINUX_ARM64_PROFILE, ARM64_BYTES),
    ] {
        let selected = toolchains.tool(profile);
        selected.ensure_unchanged().unwrap();
        assert_eq!(selected.sha256(), bytes_sha256(expected));
        assert_eq!(fs::read(selected.path()).unwrap(), expected);
        assert!(selected.resource_directory().is_some());
    }
    assert_ne!(
        toolchains.tool(LINUX_AMD64_PROFILE).path(),
        toolchains.tool(LINUX_ARM64_PROFILE).path()
    );
}

#[test]
fn linux_bazel_is_copied_into_immutable_digest_custody() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("bazel-linux");
    let bytes = b"pinned Linux Bazel fixture";
    fs::write(&path, bytes).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(EXECUTABLE_MODE)).unwrap();
    let pinned = PinnedLinuxBazel::capture(&path, &bytes_sha256(bytes)).unwrap();
    pinned.ensure_unchanged().unwrap();
    assert_eq!(pinned.sha256(), bytes_sha256(bytes));
    assert_ne!(pinned.path(), path);
}

#[test]
fn incorrect_linux_bazel_digest_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("bazel-linux");
    fs::write(&path, b"other").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(EXECUTABLE_MODE)).unwrap();
    assert!(PinnedLinuxBazel::capture(&path, &"0".repeat(SHA256_HEX_LENGTH)).is_err());
}

#[test]
fn linux_bazel_cleanup_preserves_replacement_on_drop_and_failed_capture() {
    for fail_capture in [false, true] {
        let fixture = tempfile::tempdir().unwrap();
        let source = fixture.path().join(PRIVATE_NAME);
        fs::write(&source, PINNED_BYTES).unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(EXECUTABLE_MODE)).unwrap();
        let moved = fixture.path().join(MOVED_NAME);
        let private;
        if fail_capture {
            let mut captured = None;
            let result =
                PinnedLinuxBazel::capture_with(&source, &bytes_sha256(PINNED_BYTES), |pinned| {
                    captured = Some(rebind(pinned, &moved));
                    Ok(())
                });
            assert!(result.is_err());
            private = captured.unwrap();
        } else {
            let pinned = PinnedLinuxBazel::capture(&source, &bytes_sha256(PINNED_BYTES)).unwrap();
            private = rebind(&pinned, &moved);
            assert!(pinned.ensure_unchanged().is_err());
            drop(pinned);
        }
        let sentinel = private.join(FOREIGN_NAME);
        assert_eq!(fs::read(&sentinel).unwrap(), FOREIGN_BYTES);
        assert_eq!(
            fs::metadata(&sentinel).unwrap().permissions().mode() & MODE_MASK,
            READ_ONLY_MODE
        );
        assert_eq!(
            fs::read(moved.join(yolo_release_contracts::LINUX_BAZEL_NAME)).unwrap(),
            PINNED_BYTES
        );
        fs::remove_file(sentinel).unwrap();
        fs::remove_dir(private).unwrap();
    }
}

fn rebind(pinned: &PinnedLinuxBazel, moved: &std::path::Path) -> std::path::PathBuf {
    let private = pinned.root.display_path().to_owned();
    pinned
        .root
        .set_flags(yolo_release_contracts::NO_FILE_FLAGS)
        .unwrap();
    pinned.root.set_mode(EXECUTABLE_MODE).unwrap();
    fs::rename(&private, moved).unwrap();
    fs::create_dir(&private).unwrap();
    fs::write(private.join(FOREIGN_NAME), FOREIGN_BYTES).unwrap();
    fs::set_permissions(
        private.join(FOREIGN_NAME),
        fs::Permissions::from_mode(READ_ONLY_MODE),
    )
    .unwrap();
    private
}

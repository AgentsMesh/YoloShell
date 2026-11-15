use super::support::*;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use yolo_hook_installation::{inspect, install};
mod catalog {
    pub use yolo_claude_provider::INSTALL_SPEC as CLAUDE;
    pub use yolo_codex_provider::INSTALL_SPEC as CODEX;
    pub use yolo_gemini_provider::INSTALL_SPEC as GEMINI;
    pub use yolo_grok_provider::INSTALL_SPEC as GROK;
    pub use yolo_qwen_provider::INSTALL_SPEC as QWEN;
}

const UNSAFE_FILE: u32 = 0o664;
const UNSAFE_DIRECTORY: u32 = 0o775;

#[test]
fn normal_provider_permissions_are_preserved_without_chmod() {
    let home = private_directory();
    permissions(home.path(), PUBLIC_TRAVERSE);
    let path = original(home.path(), &catalog::CLAUDE);
    permissions(&path, PUBLIC_READ);
    let preview = inspect(&catalog::CLAUDE, home.path()).unwrap();
    let installed = install(&catalog::CLAUDE, home.path(), &preview.revision).unwrap();
    assert_eq!(
        fs::metadata(home.path()).unwrap().permissions().mode() & PERMISSION_MASK,
        PUBLIC_TRAVERSE
    );
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & PERMISSION_MASK,
        PUBLIC_READ
    );
    assert_eq!(fs::read(installed.backup.unwrap()).unwrap(), ORIGINAL);
}

#[test]
fn missing_file_preview_is_bound_to_directory_identity() {
    let root = private_directory();
    let home = root.path().join("provider");
    fs::create_dir(&home).unwrap();
    let preview = inspect(&catalog::CODEX, &home).unwrap();
    fs::rename(&home, root.path().join("previous")).unwrap();
    fs::create_dir(&home).unwrap();
    assert!(install(&catalog::CODEX, &home, &preview.revision).is_err());
    assert!(!home.join(catalog::CODEX.file_name).exists());
}

#[test]
fn missing_home_preview_is_read_only_and_bound_to_parent_identity() {
    let root = private_directory();
    let parent = root.path().join("parent");
    fs::create_dir(&parent).unwrap();
    let home = parent.join("provider");
    let preview = inspect(&catalog::CLAUDE, &home).unwrap();
    assert!(!home.exists());
    fs::rename(&parent, root.path().join("previous-parent")).unwrap();
    fs::create_dir(&parent).unwrap();
    assert!(install(&catalog::CLAUDE, &home, &preview.revision).is_err());
    assert!(!home.exists());
    let fresh = inspect(&catalog::CLAUDE, &home).unwrap();
    assert!(
        install(&catalog::CLAUDE, &home, &fresh.revision)
            .unwrap()
            .changed
    );
    assert_eq!(
        fs::metadata(&home).unwrap().permissions().mode() & PERMISSION_MASK,
        DIRECTORY
    );
}

#[test]
fn preview_rejects_directory_symlink_including_dangling_symlinks() {
    let root = private_directory();
    for exists in [false, true] {
        let target = root.path().join(format!("target-{exists}"));
        if exists {
            fs::create_dir(&target).unwrap();
        }
        let home = root.path().join(format!("link-{exists}"));
        symlink(&target, &home).unwrap();
        assert!(inspect(&catalog::CODEX, &home).is_err());
        assert!(install(&catalog::CODEX, &home, "stale").is_err());
        assert!(!target.join(catalog::CODEX.file_name).exists());
    }
}

#[test]
fn group_writable_configuration_is_rejected_not_normalized() {
    let home = private_directory();
    let path = original(home.path(), &catalog::CLAUDE);
    permissions(&path, UNSAFE_FILE);
    assert!(inspect(&catalog::CLAUDE, home.path()).is_err());
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & PERMISSION_MASK,
        UNSAFE_FILE
    );
    permissions(&path, PRIVATE);
    let preview = inspect(&catalog::CLAUDE, home.path()).unwrap();
    permissions(home.path(), UNSAFE_DIRECTORY);
    assert!(install(&catalog::CLAUDE, home.path(), &preview.revision).is_err());
    assert_eq!(
        fs::metadata(home.path()).unwrap().permissions().mode() & PERMISSION_MASK,
        UNSAFE_DIRECTORY
    );
    assert_eq!(fs::read(path).unwrap(), ORIGINAL);
}

#[test]
fn a_preview_cannot_be_reused_for_another_provider_or_directory() {
    let home = private_directory();
    let preview = inspect(&catalog::CODEX, home.path()).unwrap();
    assert!(install(&catalog::CLAUDE, home.path(), &preview.revision).is_err());
    let other = private_directory();
    assert!(install(&catalog::CODEX, other.path(), &preview.revision).is_err());
}

#[test]
fn special_files_and_unsafe_ancestors_are_rejected() {
    let root = private_directory();
    let target = root.path().join(catalog::CODEX.file_name);
    fs::create_dir(&target).unwrap();
    assert!(inspect(&catalog::CODEX, root.path()).is_err());
    let home = root.path().join("provider");
    fs::create_dir(&home).unwrap();
    permissions(root.path(), UNSAFE_DIRECTORY);
    assert!(inspect(&catalog::CLAUDE, &home).is_err());
}

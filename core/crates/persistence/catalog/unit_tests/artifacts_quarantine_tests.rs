use super::artifacts_quarantine::quarantine_with_hook;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};

const ORIGINAL: &[u8] = b"corrupt catalog";
const OUTSIDE: &[u8] = b"outside must not change";
const PRIVATE_DIRECTORY_MODE: u32 = 0o700;

#[test]
fn quarantine_rejects_a_path_swap_without_touching_the_new_target() {
    let root = private_root();
    let artifact = root.path().join("catalog.sqlite");
    let moved = root.path().join("moved.sqlite");
    let outside = root.path().join("outside.sqlite");
    fs::write(&artifact, ORIGINAL).unwrap();
    fs::write(&outside, OUTSIDE).unwrap();

    let result = quarantine_with_hook(&artifact, || {
        fs::rename(&artifact, &moved)?;
        symlink(&outside, &artifact)?;
        Ok(())
    });

    assert!(result.is_err());
    assert_eq!(fs::read(&outside).unwrap(), OUTSIDE);
    assert_eq!(fs::read(&moved).unwrap(), ORIGINAL);
    assert!(
        fs::symlink_metadata(&artifact)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[test]
fn quarantine_moves_one_hardlink_without_modifying_the_other() {
    let root = private_root();
    let artifact = root.path().join("catalog.sqlite");
    let linked = root.path().join("linked.sqlite");
    fs::write(&artifact, ORIGINAL).unwrap();
    fs::hard_link(&artifact, &linked).unwrap();

    quarantine_with_hook(&artifact, || Ok(())).unwrap();
    assert!(!artifact.exists());
    assert_eq!(fs::read(&linked).unwrap(), ORIGINAL);
    assert!(fs::read_dir(root.path()).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".quarantine-")
    }));
}

#[test]
fn quarantine_detects_parent_swap_and_preserves_replacement() {
    let root = private_root();
    let parent = root.path().join("parent");
    fs::create_dir(&parent).unwrap();
    set_private(&parent);
    let artifact = parent.join("catalog.sqlite");
    fs::write(&artifact, ORIGINAL).unwrap();
    let moved = root.path().join("moved-parent");

    let result = quarantine_with_hook(&artifact, || {
        fs::rename(&parent, &moved)?;
        fs::create_dir(&parent)?;
        set_private(&parent);
        fs::write(&artifact, OUTSIDE)?;
        Ok(())
    });

    assert!(result.is_err());
    assert_eq!(fs::read(&artifact).unwrap(), OUTSIDE);
    assert!(!moved.join("catalog.sqlite").exists());
    assert!(fs::read_dir(&moved).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".quarantine-")
    }));
}

fn private_root() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    set_private(root.path());
    root
}

fn set_private(path: &std::path::Path) {
    fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE)).unwrap();
}

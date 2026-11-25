use super::super::*;
use std::ffi::CString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;

#[test]
fn missing_entry_rejects_parent_rename_and_recreation() {
    let root = tempdir().unwrap();
    let parent_path = root.path().join("parent");
    let moved_path = root.path().join("moved-parent");
    let parent = directory::PrivateDirectory::ensure(&parent_path).unwrap();
    let name = CString::new("state.json").unwrap();

    let result =
        private_file::private_file_missing::entry_is_missing_with_hook(&parent, &name, || {
            fs::rename(&parent_path, &moved_path)?;
            fs::create_dir(&parent_path)?;
            fs::set_permissions(
                &parent_path,
                fs::Permissions::from_mode(constants::PRIVATE_DIRECTORY_MODE),
            )?;
            fs::write(parent_path.join("state.json"), b"replacement")?;
            Ok(())
        });

    assert!(result.is_err());
    assert!(!moved_path.join("state.json").exists());
    assert_eq!(
        fs::read(parent_path.join("state.json")).unwrap(),
        b"replacement"
    );
}

#[test]
fn missing_entry_is_confirmed_twice_on_a_stable_parent() {
    let root = tempdir().unwrap();
    let parent = directory::PrivateDirectory::ensure(&root.path().join("parent")).unwrap();
    let name = CString::new("state.json").unwrap();
    assert!(
        private_file::private_file_missing::entry_is_missing_with_hook(&parent, &name, || Ok(()))
            .unwrap()
    );
}

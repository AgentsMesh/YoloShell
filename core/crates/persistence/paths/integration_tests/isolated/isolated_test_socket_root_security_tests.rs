use super::*;
use std::cell::RefCell;
use std::fs;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt, symlink};
use std::rc::Rc;

const DISPLACED_PREFIX: &str = "security-displaced-";
const REPLACEMENT_DIRECTORY_MODE: u32 = 0o755;

#[test]
fn post_open_replacement_is_not_secured_or_accepted() {
    let paths = Rc::new(RefCell::new(None));
    let observed = Rc::clone(&paths);
    let hook = hooks::set_fault_hook(Box::new(move |point, path| {
        if point == hooks::FaultPoint::CreationAfterOpen {
            let displaced = sibling(path);
            fs::rename(path, &displaced)?;
            fs::DirBuilder::new()
                .mode(REPLACEMENT_DIRECTORY_MODE)
                .create(path)?;
            fs::set_permissions(path, fs::Permissions::from_mode(REPLACEMENT_DIRECTORY_MODE))?;
            *observed.borrow_mut() = Some((path.to_path_buf(), displaced));
        }
        Ok(())
    }));

    assert!(IsolatedTestSocketRoot::create().is_err());

    drop(hook);
    let (replacement, displaced) = paths.borrow().clone().unwrap();
    assert_eq!(mode(&replacement), REPLACEMENT_DIRECTORY_MODE);
    assert_eq!(mode(&displaced), PRIVATE_DIRECTORY_MODE);
    fs::remove_dir(replacement).unwrap();
    fs::remove_dir(displaced).unwrap();
}

#[test]
fn pre_open_symlink_replacement_is_rejected_without_following_target() {
    let temporary = tempfile::tempdir().unwrap();
    fs::set_permissions(
        temporary.path(),
        fs::Permissions::from_mode(REPLACEMENT_DIRECTORY_MODE),
    )
    .unwrap();
    let paths = Rc::new(RefCell::new(None));
    let observed = Rc::clone(&paths);
    let target = temporary.path().to_path_buf();
    let target_for_hook = target.clone();
    let hook = hooks::set_fault_hook(Box::new(move |point, path| {
        if point == hooks::FaultPoint::CreationBeforeOpen {
            let displaced = sibling(path);
            fs::rename(path, &displaced)?;
            symlink(&target_for_hook, path)?;
            *observed.borrow_mut() = Some((path.to_path_buf(), displaced));
        }
        Ok(())
    }));

    assert!(IsolatedTestSocketRoot::create().is_err());

    drop(hook);
    let (replacement, displaced) = paths.borrow().clone().unwrap();
    assert_eq!(mode(&target), REPLACEMENT_DIRECTORY_MODE);
    assert!(
        replacement
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink()
    );
    fs::remove_file(replacement).unwrap();
    fs::remove_dir(displaced).unwrap();
}

fn sibling(path: &Path) -> PathBuf {
    path.with_file_name(format!(
        "{DISPLACED_PREFIX}{}",
        uuid::Uuid::new_v4().simple()
    ))
}

fn mode(path: &Path) -> u32 {
    path.metadata().unwrap().mode() & PERMISSION_MODE_MASK
}

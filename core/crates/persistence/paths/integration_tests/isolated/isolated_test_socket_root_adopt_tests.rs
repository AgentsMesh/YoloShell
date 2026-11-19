use super::*;
use std::cell::Cell;
use std::fs::{self, File};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::Path;
use std::rc::Rc;

const OUTSIDE_PATH: &str = "/private/yolo-outside/yn-12345678";
const UPPERCASE_NAME: &str = "yn-ABCDEF12";
const SHORT_NAME: &str = "yn-1234";
const REPLACEMENT_PREFIX: &str = "adopt-displaced-";
const FOREIGN_ENTRY: &str = "foreign";
const PUBLIC_DIRECTORY_MODE: u32 = 0o755;

#[test]
fn adoption_rejects_paths_outside_the_exact_lowercase_namespace() {
    for path in [
        Path::new(OUTSIDE_PATH).to_path_buf(),
        Path::new(ISOLATED_TEST_SOCKET_PARENT).join(UPPERCASE_NAME),
        Path::new(ISOLATED_TEST_SOCKET_PARENT).join(SHORT_NAME),
    ] {
        assert!(IsolatedTestSocketRoot::adopt(&path).is_err());
    }
}

#[test]
fn adoption_rejects_non_directory_symlink_and_non_private_roots() {
    for replacement in [Replacement::File, Replacement::Symlink] {
        assert_replacement_rejected(replacement);
    }
    let mut root = IsolatedTestSocketRoot::create().unwrap();
    let path = root.path().to_path_buf();
    fs::set_permissions(&path, fs::Permissions::from_mode(PUBLIC_DIRECTORY_MODE)).unwrap();
    assert!(IsolatedTestSocketRoot::adopt(&path).is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE)).unwrap();
    root.finalize().unwrap();
}

#[test]
fn adoption_requires_an_empty_root() {
    let mut root = IsolatedTestSocketRoot::create().unwrap();
    let foreign = root.path().join(FOREIGN_ENTRY);
    File::create(&foreign).unwrap();
    assert!(IsolatedTestSocketRoot::adopt(root.path()).is_err());
    fs::remove_file(foreign).unwrap();
    root.finalize().unwrap();
}

#[test]
fn adoption_fences_a_replaced_root_without_touching_the_replacement() {
    let mut root = IsolatedTestSocketRoot::create().unwrap();
    let active = root.path().to_path_buf();
    let displaced = active.with_file_name(format!(
        "{REPLACEMENT_PREFIX}{}",
        uuid::Uuid::new_v4().simple()
    ));
    let replaced = Rc::new(Cell::new(false));
    let observed = Rc::clone(&replaced);
    let active_hook = active.clone();
    let displaced_hook = displaced.clone();
    let hook = hooks::set_fault_hook(Box::new(move |point, _path| {
        if point == hooks::FaultPoint::AdoptBeforeVerify && !observed.replace(true) {
            fs::rename(&active_hook, &displaced_hook)?;
            fs::create_dir(&active_hook)?;
            fs::set_permissions(
                &active_hook,
                fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
            )?;
        }
        Ok(())
    }));
    assert!(IsolatedTestSocketRoot::adopt(&active).is_err());
    assert!(active.is_dir() && replaced.get());
    drop(hook);
    fs::remove_dir(&active).unwrap();
    fs::rename(&displaced, &active).unwrap();
    root.finalize().unwrap();
}

#[test]
fn concurrent_custodians_remove_the_root_at_most_once() {
    let mut first = IsolatedTestSocketRoot::create().unwrap();
    let path = first.path().to_path_buf();
    let mut second = IsolatedTestSocketRoot::adopt(&path).unwrap();
    let first = std::thread::spawn(move || first.finalize());
    let second = std::thread::spawn(move || second.finalize());
    let outcomes = [
        first.join().unwrap().is_ok(),
        second.join().unwrap().is_ok(),
    ];
    assert_eq!(outcomes.into_iter().filter(|outcome| *outcome).count(), 1);
    assert!(!path.exists());
}

#[derive(Clone, Copy)]
enum Replacement {
    File,
    Symlink,
}

fn assert_replacement_rejected(replacement: Replacement) {
    let mut root = IsolatedTestSocketRoot::create().unwrap();
    let path = root.path().to_path_buf();
    root.finalize().unwrap();
    match replacement {
        Replacement::File => drop(File::create(&path).unwrap()),
        Replacement::Symlink => symlink(std::env::temp_dir(), &path).unwrap(),
    }
    assert!(IsolatedTestSocketRoot::adopt(&path).is_err());
    fs::remove_file(path).unwrap();
}

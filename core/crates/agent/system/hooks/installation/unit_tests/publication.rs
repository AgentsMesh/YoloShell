use super::*;
use crate::filesystem_test_support::*;
use std::cell::RefCell;
use std::fs;

thread_local! {
    static BEFORE_EXCHANGE: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
}

pub(super) fn before_exchange() {
    if let Some(action) = BEFORE_EXCHANGE.with(|hook| hook.borrow_mut().take()) {
        action();
    }
}

fn before(action: impl FnOnce() + 'static) {
    BEFORE_EXCHANGE.with(|hook| *hook.borrow_mut() = Some(Box::new(action)));
}

#[test]
fn contested_exchange_restores_concurrent_writer_and_preserves_backup() {
    let (_home, directory, snapshot) = fixture(true);
    let backup = backup(&directory, CODEX_FILE, &snapshot).unwrap().unwrap();
    let target = directory.path().join(CODEX_FILE);
    let contested = target.clone();
    before(move || fs::write(contested, CONCURRENT).unwrap());
    assert!(write(&directory, CODEX_FILE, &snapshot, UPDATED).is_err());
    assert_eq!(fs::read(target).unwrap(), CONCURRENT);
    assert_eq!(fs::read(backup).unwrap(), ORIGINAL);
}

#[test]
fn absent_target_publication_never_overwrites_a_concurrent_creation() {
    let (_home, directory, snapshot) = fixture(false);
    let target = directory.path().join(CODEX_FILE);
    let contested = target.clone();
    before(move || fs::write(contested, CONCURRENT).unwrap());
    assert!(write(&directory, CODEX_FILE, &snapshot, UPDATED).is_err());
    assert_eq!(fs::read(target).unwrap(), CONCURRENT);
}

#[test]
fn directory_replacement_after_validation_cannot_write_into_replacement() {
    let (_home, directory, snapshot) = fixture(true);
    let target = directory.path().to_path_buf();
    let retained = target.with_extension(uuid::Uuid::new_v4().to_string());
    let replacement = target.clone();
    let retained_for_hook = retained.clone();
    before(move || {
        fs::rename(&replacement, retained_for_hook).unwrap();
        fs::create_dir(&replacement).unwrap();
        fs::write(replacement.join(CODEX_FILE), CONCURRENT).unwrap();
    });
    assert!(write(&directory, CODEX_FILE, &snapshot, UPDATED).is_err());
    assert_eq!(fs::read(target.join(CODEX_FILE)).unwrap(), CONCURRENT);
    // The retained directory still contains the original bytes under the displaced staging name.
    assert!(
        fs::read_dir(&retained)
            .unwrap()
            .any(|entry| fs::read(entry.unwrap().path()).ok().as_deref() == Some(ORIGINAL))
    );
}

#[test]
fn backup_collision_is_not_replaced_or_accepted() {
    let (_home, directory, snapshot) = fixture(true);
    let name = format!("{BACKUP_PREFIX}{}{BACKUP_SUFFIX}", snapshot.revision);
    let collision = directory.path().join(name);
    fs::write(&collision, CONCURRENT).unwrap();
    assert!(backup(&directory, CODEX_FILE, &snapshot).is_err());
    assert_eq!(fs::read(collision).unwrap(), CONCURRENT);
    assert_eq!(
        fs::read(directory.path().join(CODEX_FILE)).unwrap(),
        ORIGINAL
    );
}

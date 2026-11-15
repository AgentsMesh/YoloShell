use super::secure_vfs_file::ArtifactKind;
use anyhow::Result;
use std::cell::RefCell;
use std::path::Path;

type TestAction = Box<dyn FnOnce(&Path) -> Result<()>>;

struct TestHook {
    kind: ArtifactKind,
    action: TestAction,
}

thread_local! {
    static TEST_HOOK: RefCell<Option<TestHook>> = const { RefCell::new(None) };
    static ALIAS_TEST_HOOK: RefCell<Option<TestHook>> = const { RefCell::new(None) };
}

pub(super) fn set<F>(kind: ArtifactKind, action: F)
where
    F: FnOnce(&Path) -> Result<()> + 'static,
{
    TEST_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(TestHook {
            kind,
            action: Box::new(action),
        });
    });
}

pub(super) fn run(kind: ArtifactKind, path: &Path) {
    run_hook(&TEST_HOOK, kind, path);
}

pub(super) fn set_alias<F>(kind: ArtifactKind, action: F)
where
    F: FnOnce(&Path) -> Result<()> + 'static,
{
    ALIAS_TEST_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(TestHook {
            kind,
            action: Box::new(action),
        });
    });
}

pub(super) fn run_alias(kind: ArtifactKind, path: &Path) {
    run_hook(&ALIAS_TEST_HOOK, kind, path);
}

fn run_hook(
    slot: &'static std::thread::LocalKey<RefCell<Option<TestHook>>>,
    kind: ArtifactKind,
    path: &Path,
) {
    slot.with(|hook| {
        let should_run = hook.borrow().as_ref().is_some_and(|hook| hook.kind == kind);
        if should_run
            && let Some(hook) = hook.borrow_mut().take()
            && let Err(error) = (hook.action)(path)
        {
            super::secure_vfs::record_violation(format!("SQLite VFS test hook failed: {error:#}"));
        }
    });
}

const EMPTY_SQLITE_BYTES: &[u8] = b"";
const EXTERNAL_SENTINEL: &[u8] = b"external SQLite sentinel";

#[derive(Clone, Copy)]
enum Replacement {
    HardLink,
    Symlink,
}

#[test]
fn main_xopen_rejects_path_swap_without_touching_external_inode() {
    assert_path_swap_is_safe(ArtifactKind::Main, Replacement::HardLink);
}

#[test]
fn journal_xopen_rejects_path_swap_without_touching_external_inode() {
    assert_path_swap_is_safe(ArtifactKind::Journal, Replacement::HardLink);
}

#[test]
fn wal_xopen_rejects_path_swap_without_touching_external_inode() {
    assert_path_swap_is_safe(ArtifactKind::Wal, Replacement::HardLink);
}

#[test]
fn shm_map_rejects_path_swap_without_touching_external_inode() {
    assert_path_swap_is_safe(ArtifactKind::Shm, Replacement::HardLink);
}

#[test]
fn every_artifact_rejects_a_symlink_swap() {
    for kind in [
        ArtifactKind::Main,
        ArtifactKind::Journal,
        ArtifactKind::Wal,
        ArtifactKind::Shm,
    ] {
        assert_path_swap_is_safe(kind, Replacement::Symlink);
    }
}

#[test]
fn missing_descriptor_path_is_an_operational_error() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite");
    std::fs::write(&path, EMPTY_SQLITE_BYTES).unwrap();
    let name = CString::new(path.as_os_str().as_bytes()).unwrap();
    let flags = rusqlite::ffi::SQLITE_OPEN_MAIN_DB | rusqlite::ffi::SQLITE_OPEN_READWRITE;
    let descriptor = super::secure_vfs_file::SecureDescriptor::open(&name, flags).unwrap();
    std::fs::remove_file(path).unwrap();

    let error = descriptor.verify_current().unwrap_err();

    assert!(!super::rebuild_reason::is_rebuildable(&error));
    assert!(error.chain().any(|cause| cause.is::<std::io::Error>()));
}

fn assert_path_swap_is_safe(kind: ArtifactKind, replacement: Replacement) {
    let directory = tempfile::tempdir().unwrap();
    let catalog = directory.path().join("catalog.sqlite");
    let outside = directory.path().join("outside.sqlite");
    let displaced = directory.path().join("displaced.sqlite");
    std::fs::write(&outside, EXTERNAL_SENTINEL).unwrap();
    let outside_for_hook = outside.clone();
    set(kind, move |path| {
        std::fs::rename(path, &displaced)?;
        match replacement {
            Replacement::HardLink => std::fs::hard_link(&outside_for_hook, path)?,
            Replacement::Symlink => std::os::unix::fs::symlink(&outside_for_hook, path)?,
        }
        Ok(())
    });

    let error = super::Catalog::open(&catalog).err().unwrap();

    assert!(super::rebuild_reason::is_rebuildable(&error));
    assert_eq!(std::fs::read(outside).unwrap(), EXTERNAL_SENTINEL);
}

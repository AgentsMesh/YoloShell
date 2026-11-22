use super::secure_vfs_alias_init::PublishedAlias;
use super::secure_vfs_alias_io::ALIAS_ACTIVE_MODE;
use super::secure_vfs_file::{ArtifactKind, SecureDescriptor};
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const SENTINEL: &[u8] = b"replacement must survive";
const CATALOG_FILE: &str = "catalog.sqlite";
const MOVED_ALIAS_DIRECTORY: &str = "moved-alias";
const SENTINEL_FILE: &str = "sentinel";

type PublishHook = Box<dyn FnOnce(&Path, &Path) -> anyhow::Result<()>>;
type CleanupHook = Box<dyn FnOnce(&Path)>;

thread_local! {
    static PUBLISH_HOOK: std::cell::RefCell<Option<PublishHook>> = const {
        std::cell::RefCell::new(None)
    };
    static CLEANUP_HOOK: std::cell::RefCell<Option<CleanupHook>> = const {
        std::cell::RefCell::new(None)
    };
}

pub(super) fn set_publish_hook<F>(hook: F)
where
    F: FnOnce(&Path, &Path) -> anyhow::Result<()> + 'static,
{
    PUBLISH_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

pub(super) fn run_publish_hook(source: &Path, target: &Path) -> anyhow::Result<()> {
    PUBLISH_HOOK.with(|slot| match slot.borrow_mut().take() {
        Some(hook) => hook(source, target),
        None => Ok(()),
    })
}

pub(super) fn set_cleanup_hook<F>(hook: F)
where
    F: FnOnce(&Path) + 'static,
{
    CLEANUP_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

pub(super) fn run_cleanup_hook(path: &Path) {
    CLEANUP_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook(path);
        }
    });
}

#[test]
fn published_alias_drop_does_not_clean_or_remove_a_replacement() {
    let root = tempfile::tempdir().unwrap();
    let artifact = root.path().join(CATALOG_FILE);
    std::fs::write(&artifact, []).unwrap();
    let descriptor = descriptor(&artifact);
    let alias = PublishedAlias::create(&descriptor, ArtifactKind::Main).unwrap();
    let published = alias.path().to_path_buf();
    let moved = root.path().join(MOVED_ALIAS_DIRECTORY);
    std::fs::rename(&published, &moved).unwrap();
    private_directory(&published);
    let sentinel = published.join(SENTINEL_FILE);
    std::fs::write(&sentinel, SENTINEL).unwrap();

    drop(alias);

    assert_eq!(std::fs::read(sentinel).unwrap(), SENTINEL);
    assert!(published.is_dir());
}

#[test]
fn stale_alias_cleanup_rejects_a_scanned_entry_replacement() {
    let root = tempfile::tempdir().unwrap();
    crate::paths::ensure_private_parent(&root.path().join(CATALOG_FILE)).unwrap();
    let alias = root.path().join(format!(
        "{}{}",
        super::secure_vfs_alias_io::ALIAS_PREFIX,
        uuid::Uuid::new_v4()
    ));
    private_directory(&alias);
    let moved = root.path().join(MOVED_ALIAS_DIRECTORY);
    let sentinel = alias.join(super::secure_vfs_alias_io::ALIAS_MAIN_NAME);

    super::secure_vfs_cleanup::cleanup_with_hook(&root.path().join(CATALOG_FILE), |scanned| {
        if scanned == alias {
            std::fs::rename(&alias, &moved)?;
            private_directory(&alias);
            std::fs::write(&sentinel, SENTINEL)?;
        }
        Ok(())
    })
    .unwrap();

    assert_eq!(std::fs::read(sentinel).unwrap(), SENTINEL);
    assert!(alias.is_dir());
}

#[test]
fn alias_publish_rejects_a_source_replacement() {
    let root = tempfile::tempdir().unwrap();
    let artifact = root.path().join(CATALOG_FILE);
    std::fs::write(&artifact, []).unwrap();
    let descriptor = descriptor(&artifact);
    let moved = root.path().join(MOVED_ALIAS_DIRECTORY);
    let observed = Arc::new(Mutex::new(None));
    let captured = Arc::clone(&observed);
    set_publish_hook(move |source, _| {
        std::fs::rename(source, &moved)?;
        private_directory(source);
        let sentinel = source.join(SENTINEL_FILE);
        std::fs::write(&sentinel, SENTINEL)?;
        *captured.lock().unwrap() = Some((source.to_path_buf(), sentinel));
        Ok(())
    });

    let result = PublishedAlias::create(&descriptor, ArtifactKind::Main);
    let (source, sentinel) = take_observed(&observed);

    assert!(result.is_err());
    assert!(source.is_dir());
    assert_eq!(std::fs::read(sentinel).unwrap(), SENTINEL);
}

#[test]
fn alias_publish_preserves_a_preexisting_target() {
    let root = tempfile::tempdir().unwrap();
    let artifact = root.path().join(CATALOG_FILE);
    std::fs::write(&artifact, []).unwrap();
    let descriptor = descriptor(&artifact);
    let observed = Arc::new(Mutex::new(None));
    let captured = Arc::clone(&observed);
    set_publish_hook(move |source, target| {
        private_directory(target);
        let sentinel = target.join(SENTINEL_FILE);
        std::fs::write(&sentinel, SENTINEL)?;
        *captured.lock().unwrap() = Some((source.to_path_buf(), sentinel));
        Ok(())
    });

    let result = PublishedAlias::create(&descriptor, ArtifactKind::Main);
    let (source, sentinel) = take_observed(&observed);

    assert!(result.is_err());
    assert!(!source.exists());
    assert_eq!(std::fs::read(sentinel).unwrap(), SENTINEL);
}

#[test]
fn alias_cleanup_rechecks_identity_before_unlink() {
    let root = tempfile::tempdir().unwrap();
    let artifact = root.path().join(CATALOG_FILE);
    std::fs::write(&artifact, []).unwrap();
    let descriptor = descriptor(&artifact);
    let alias = PublishedAlias::create(&descriptor, ArtifactKind::Main).unwrap();
    let published = alias.path().to_path_buf();
    let sentinel = published.join(SENTINEL_FILE);
    let moved = root.path().join(MOVED_ALIAS_DIRECTORY);
    set_cleanup_hook(move |path| {
        std::fs::rename(path, moved).unwrap();
        private_directory(path);
        std::fs::write(path.join(SENTINEL_FILE), SENTINEL).unwrap();
    });

    drop(alias);

    assert!(published.is_dir());
    assert_eq!(std::fs::read(sentinel).unwrap(), SENTINEL);
}

fn take_observed(observed: &Arc<Mutex<Option<(PathBuf, PathBuf)>>>) -> (PathBuf, PathBuf) {
    observed.lock().unwrap().take().unwrap()
}

fn private_directory(path: &Path) {
    let mut builder = std::fs::DirBuilder::new();
    builder.mode(ALIAS_ACTIVE_MODE).create(path).unwrap();
}

fn descriptor(path: &Path) -> SecureDescriptor {
    let name = CString::new(path.as_os_str().as_bytes()).unwrap();
    let flags = rusqlite::ffi::SQLITE_OPEN_MAIN_DB | rusqlite::ffi::SQLITE_OPEN_READWRITE;
    SecureDescriptor::open(&name, flags).unwrap()
}

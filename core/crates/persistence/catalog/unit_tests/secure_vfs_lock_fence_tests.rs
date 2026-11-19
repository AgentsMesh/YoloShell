use super::secure_vfs_alias_io::{
    ALIAS_ACTIVE_MODE, ALIAS_LOCK_NAME, ALIAS_PREFIX, create_lock_file,
};
use crate::paths::constants::PRIVATE_FILE_MODE;
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

const ARTIFACT_FILE_NAME: &str = "catalog.sqlite";
const MOVED_ALIAS_NAME: &str = "moved-alias";
const MOVED_LOCK_NAME: &str = "moved-owner.lock";
const REPLACEMENT_FILE_NAME: &str = "replacement";
const REPLACEMENT_BYTES: &[u8] = b"replacement must survive";

#[test]
fn owner_lock_creation_rejects_a_post_lock_entry_replacement() {
    let fixture = Fixture::new();
    replace_lock_after_acquire(&fixture.alias);

    let directory = super::secure_vfs_alias_directory::open_verified(&fixture.alias).unwrap();
    let result = create_lock_file(&directory, &fixture.alias);

    assert!(result.is_err());
    assert_eq!(
        std::fs::read(fixture.alias.join(ALIAS_LOCK_NAME)).unwrap(),
        REPLACEMENT_BYTES
    );
}

#[test]
fn stale_cleanup_preserves_a_post_lock_entry_replacement() {
    let fixture = Fixture::with_unlocked_owner();
    replace_lock_after_acquire(&fixture.alias);

    super::secure_vfs_cleanup::cleanup_stale_aliases(&fixture.artifact).unwrap();

    assert_eq!(
        std::fs::read(fixture.alias.join(ALIAS_LOCK_NAME)).unwrap(),
        REPLACEMENT_BYTES
    );
}

#[test]
fn stale_cleanup_preserves_a_post_lock_directory_replacement() {
    let fixture = Fixture::with_unlocked_owner();
    let alias = fixture.alias.clone();
    let moved = fixture.root.path().join(MOVED_ALIAS_NAME);
    super::secure_vfs_lock_hooks::set(move |_| {
        std::fs::rename(&alias, moved)?;
        private_directory(&alias);
        std::fs::write(alias.join(REPLACEMENT_FILE_NAME), REPLACEMENT_BYTES)?;
        Ok(())
    });

    super::secure_vfs_cleanup::cleanup_stale_aliases(&fixture.artifact).unwrap();

    assert_eq!(
        std::fs::read(fixture.alias.join(REPLACEMENT_FILE_NAME)).unwrap(),
        REPLACEMENT_BYTES
    );
}

fn replace_lock_after_acquire(alias: &Path) {
    let alias = alias.to_path_buf();
    super::secure_vfs_lock_hooks::set(move |_| {
        let lock = alias.join(ALIAS_LOCK_NAME);
        std::fs::rename(&lock, alias.join(MOVED_LOCK_NAME))?;
        let mut replacement = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(PRIVATE_FILE_MODE)
            .open(lock)?;
        replacement.write_all(REPLACEMENT_BYTES)?;
        Ok(())
    });
}

struct Fixture {
    root: tempfile::TempDir,
    artifact: PathBuf,
    alias: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let artifact = root.path().join(ARTIFACT_FILE_NAME);
        crate::paths::ensure_private_parent(&artifact).unwrap();
        let alias = root
            .path()
            .join(format!("{ALIAS_PREFIX}{}", uuid::Uuid::new_v4()));
        private_directory(&alias);
        Self {
            root,
            artifact,
            alias,
        }
    }

    fn with_unlocked_owner() -> Self {
        let fixture = Self::new();
        let directory = super::secure_vfs_alias_directory::open_verified(&fixture.alias).unwrap();
        drop(create_lock_file(&directory, &fixture.alias).unwrap());
        fixture
    }
}

fn private_directory(path: &Path) {
    let mut builder = std::fs::DirBuilder::new();
    builder.mode(ALIAS_ACTIVE_MODE).create(path).unwrap();
}

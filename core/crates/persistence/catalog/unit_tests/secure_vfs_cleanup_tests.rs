const CLEANUP_SCAN_TEST_LIMIT: usize = 32;
const CLEANUP_SCAN_OVERFLOW_ENTRIES: usize = CLEANUP_SCAN_TEST_LIMIT + 1;
const CATALOG_FILE: &str = "catalog.sqlite";
const INITIALIZING_ENTRY_BYTES: &[u8] = b"keep";
const INVALID_ALIAS_SUFFIX: &str = "not-a-uuid";
const MOVED_PARENT_DIRECTORY: &str = "moved-parent";
const PRIVATE_PARENT_DIRECTORY: &str = "catalog-parent";
const REPLACEMENT_SENTINEL: &[u8] = b"replacement parent must survive";
const SENTINEL_FILE: &str = "sentinel";
const UNKNOWN_ALIAS_ENTRY: &str = "unknown-entry";
const FIRST_ALIAS_ID: u128 = 1;
const SECOND_ALIAS_ID: u128 = 2;
const NON_UTF8_BYTE: u8 = 0xff;

#[derive(Clone, Copy)]
enum StaleBlocker {
    KnownDirectory,
    UnknownFile,
}

#[test]
fn alias_cleanup_counts_every_malformed_entry_without_deleting_it() {
    let directory = tempfile::tempdir().unwrap();
    let artifact = directory.path().join(CATALOG_FILE);
    for index in 0..CLEANUP_SCAN_OVERFLOW_ENTRIES {
        let name = format!(
            "{}{INVALID_ALIAS_SUFFIX}-{index}",
            super::secure_vfs_alias_io::ALIAS_PREFIX
        );
        std::fs::write(directory.path().join(name), INITIALIZING_ENTRY_BYTES).unwrap();
    }

    assert!(
        super::secure_vfs_cleanup::cleanup_with_limit(&artifact, CLEANUP_SCAN_TEST_LIMIT).is_err()
    );
    assert_eq!(
        std::fs::read_dir(directory.path()).unwrap().count(),
        CLEANUP_SCAN_OVERFLOW_ENTRIES
    );
}

#[test]
fn initialization_lock_outlives_pid_reuse_and_controls_cleanup() {
    let directory = tempfile::tempdir().unwrap();
    let artifact = directory.path().join(CATALOG_FILE);
    let initializing = directory.path().join(format!(
        "{}{}-{}",
        super::secure_vfs_alias_io::ALIAS_INITIALIZING_PREFIX,
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    private_directory(&initializing);
    let opened = super::secure_vfs_alias_directory::open_verified(&initializing).unwrap();
    let lock = super::secure_vfs_alias_io::create_lock_file(&opened, &initializing).unwrap();

    super::secure_vfs_cleanup::cleanup_stale_aliases(&artifact).unwrap();
    assert!(initializing.exists());
    drop(lock);
    super::secure_vfs_cleanup::cleanup_stale_aliases(&artifact).unwrap();
    assert!(!initializing.exists());
}

#[test]
fn impossible_initialization_process_identity_is_not_treated_as_stale() {
    let directory = tempfile::tempdir().unwrap();
    let artifact = directory.path().join(CATALOG_FILE);
    let initializing = directory.path().join(format!(
        "{}0-{}",
        super::secure_vfs_alias_io::ALIAS_INITIALIZING_PREFIX,
        uuid::Uuid::new_v4()
    ));
    private_directory(&initializing);

    super::secure_vfs_cleanup::cleanup_stale_aliases(&artifact).unwrap();
    assert!(initializing.exists());
}

#[test]
fn stale_cleanup_mutates_only_the_retained_parent_after_replacement() {
    let root = tempfile::tempdir().unwrap();
    let parent = root.path().join(PRIVATE_PARENT_DIRECTORY);
    crate::paths::directory::PrivateDirectory::ensure(&parent).unwrap();
    let artifact = parent.join(CATALOG_FILE);
    std::fs::write(&artifact, []).unwrap();
    let name = format!(
        "{}{}",
        super::secure_vfs_alias_io::ALIAS_PREFIX,
        uuid::Uuid::new_v4()
    );
    let alias = parent.join(&name);
    private_directory(&alias);
    let opened = super::secure_vfs_alias_directory::open_verified(&alias).unwrap();
    let lock = super::secure_vfs_alias_io::create_lock_file(&opened, &alias).unwrap();
    drop(lock);
    let moved_parent = root.path().join(MOVED_PARENT_DIRECTORY);
    let sentinel = alias.join(SENTINEL_FILE);

    super::secure_vfs_cleanup::cleanup_with_hook(&artifact, |scanned| {
        if scanned == alias {
            std::fs::rename(&parent, &moved_parent)?;
            private_directory(&parent);
            private_directory(&alias);
            std::fs::write(&sentinel, REPLACEMENT_SENTINEL)?;
        }
        Ok(())
    })
    .unwrap();

    assert_eq!(std::fs::read(&sentinel).unwrap(), REPLACEMENT_SENTINEL);
    assert!(!moved_parent.join(name).exists());
}

#[test]
fn named_cleanup_rejects_untrusted_and_missing_candidate_shapes() {
    use std::os::unix::ffi::OsStringExt;

    let root = tempfile::tempdir().unwrap();
    let parent = crate::paths::directory::PrivateDirectory::ensure(
        &root.path().join(PRIVATE_PARENT_DIRECTORY),
    )
    .unwrap();
    let regular_name = alias_name(FIRST_ALIAS_ID);
    std::fs::write(parent.path().join(&regular_name), INITIALIZING_ENTRY_BYTES).unwrap();
    let missing_name = alias_name(SECOND_ALIAS_ID);
    let non_utf8 = std::ffi::OsString::from_vec(vec![NON_UTF8_BYTE]);

    for name in [
        non_utf8.as_os_str(),
        std::ffi::OsStr::new(INVALID_ALIAS_SUFFIX),
        std::ffi::OsStr::new(&regular_name),
        std::ffi::OsStr::new(&missing_name),
    ] {
        assert!(
            !super::secure_vfs_cleanup::cleanup_named_candidate(&parent, name).unwrap(),
            "unexpected cleanup for {name:?}"
        );
    }
}

#[test]
fn strict_cleanup_preserves_known_directories_and_unknown_entries() {
    for (id, blocker) in [
        (FIRST_ALIAS_ID, StaleBlocker::KnownDirectory),
        (SECOND_ALIAS_ID, StaleBlocker::UnknownFile),
    ] {
        let root = tempfile::tempdir().unwrap();
        let parent = crate::paths::directory::PrivateDirectory::ensure(
            &root.path().join(PRIVATE_PARENT_DIRECTORY),
        )
        .unwrap();
        let name = alias_name(id);
        let alias = parent.path().join(&name);
        private_directory(&alias);
        let opened = super::secure_vfs_alias_directory::open_verified(&alias).unwrap();
        drop(super::secure_vfs_alias_io::create_lock_file(&opened, &alias).unwrap());
        let blocker = match blocker {
            StaleBlocker::KnownDirectory => {
                let path = alias.join(super::secure_vfs_alias_io::ALIAS_MAIN_NAME);
                private_directory(&path);
                path
            }
            StaleBlocker::UnknownFile => {
                let path = alias.join(UNKNOWN_ALIAS_ENTRY);
                std::fs::write(&path, INITIALIZING_ENTRY_BYTES).unwrap();
                path
            }
        };

        assert!(
            !super::secure_vfs_cleanup::cleanup_named_candidate(
                &parent,
                std::ffi::OsStr::new(&name),
            )
            .unwrap()
        );
        assert!(blocker.exists());
    }
}

fn alias_name(id: u128) -> String {
    format!(
        "{}{}",
        super::secure_vfs_alias_io::ALIAS_PREFIX,
        uuid::Uuid::from_u128(id)
    )
}

fn private_directory(path: &std::path::Path) {
    use std::os::unix::fs::DirBuilderExt;
    let mut builder = std::fs::DirBuilder::new();
    builder
        .mode(super::secure_vfs_alias_io::ALIAS_ACTIVE_MODE)
        .create(path)
        .unwrap();
}

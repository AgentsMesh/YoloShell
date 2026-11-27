use super::retained_namespace::RetainedCatalogNamespace;
use super::retained_test_fixture::RetainedFixture;
use super::{Catalog, constants};
use std::os::unix::fs::{DirBuilderExt, symlink};
use std::path::Path;

const POISON_ENTRY_PREFIX: &str = "foreign-unsafe-entry-";
const POISON_ENTRY_BYTES: &[u8] = b"untrusted";
const POISON_ENTRY_COUNT: usize = constants::MAX_SECURE_VFS_PARENT_ENTRIES + 1;

#[test]
fn poisoned_catalog_shard_cannot_block_another_catalog_open() {
    let first = RetainedFixture::new();
    let second = RetainedFixture::new();
    let first_namespace = RetainedCatalogNamespace::register(first.entry()).unwrap();
    let second_namespace = RetainedCatalogNamespace::register(second.entry()).unwrap();
    assert_ne!(
        first_namespace.alias_parent(),
        second_namespace.alias_parent()
    );
    poison(first_namespace.alias_parent());

    assert!(first_namespace.cleanup_aliases().is_err());
    let second_path = second_namespace.path().to_path_buf();
    let opened = Catalog::open_with_namespace(&second_path, Some(second_namespace));

    assert!(opened.is_ok());
}

#[test]
fn unsafe_alias_lock_cannot_block_another_catalog_open() {
    let first = RetainedFixture::new();
    let second = RetainedFixture::new();
    let first_namespace = RetainedCatalogNamespace::register(first.entry()).unwrap();
    let second_namespace = RetainedCatalogNamespace::register(second.entry()).unwrap();
    let alias = first_namespace.alias_parent().join(format!(
        "{}{}",
        super::secure_vfs_alias_io::ALIAS_PREFIX,
        uuid::Uuid::new_v4()
    ));
    private_directory(&alias);
    private_directory(&alias.join(super::secure_vfs_alias_io::ALIAS_LOCK_NAME));

    first_namespace.cleanup_aliases().unwrap();
    assert!(alias.exists());
    let second_path = second_namespace.path().to_path_buf();
    let opened = Catalog::open_with_namespace(&second_path, Some(second_namespace));

    assert!(opened.is_ok());
}

#[test]
fn retained_sidecars_share_the_main_catalog_shard() {
    let fixture = RetainedFixture::new();
    let namespace = RetainedCatalogNamespace::register(fixture.entry()).unwrap();
    let expected = namespace.alias_parent();

    for suffix in [
        "",
        constants::SQLITE_JOURNAL_SUFFIX,
        constants::SQLITE_WAL_SUFFIX,
        constants::SQLITE_SHM_SUFFIX,
    ] {
        let artifact =
            super::retained_namespace::resolve(&append(&fixture.catalog, suffix)).unwrap();
        assert_eq!(artifact.alias_parent().unwrap().path(), expected);
    }
}

#[test]
fn canonical_catalog_paths_select_the_same_shard() {
    let root = tempfile::tempdir().unwrap();
    let physical = root.path().join("physical");
    let alias = root.path().join("alias");
    std::fs::create_dir(&physical).unwrap();
    symlink(&physical, &alias).unwrap();

    let physical_shard =
        crate::coordination::catalog_alias_shard_name(&physical.join("catalog.sqlite")).unwrap();
    let alias_shard =
        crate::coordination::catalog_alias_shard_name(&alias.join("catalog.sqlite")).unwrap();

    assert_eq!(physical_shard, alias_shard);
}

fn poison(directory: &Path) {
    for index in 0..POISON_ENTRY_COUNT {
        std::fs::write(
            directory.join(format!("{POISON_ENTRY_PREFIX}{index}")),
            POISON_ENTRY_BYTES,
        )
        .unwrap();
    }
}

fn private_directory(path: &Path) {
    let mut builder = std::fs::DirBuilder::new();
    builder
        .mode(super::secure_vfs_alias_io::ALIAS_ACTIVE_MODE)
        .create(path)
        .unwrap();
}

fn append(path: &Path, suffix: &str) -> std::path::PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    name.into()
}

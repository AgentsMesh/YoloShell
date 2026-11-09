use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::PathBuf;
use yolo_catalog::Catalog;
use yolo_paths::directory::PrivateDirectory;
use yolo_paths::private_file::PrivateFileEntry;

const SECOND_FILESYSTEM: &str = "/dev/shm";
const TEMP_PREFIX: &str = "yolo-catalog-filesystem-";
const RUNTIME: &str = "runtime";
const DISPLACED_RUNTIME: &str = "displaced-runtime";
const CATALOG: &str = "catalog.sqlite";
const LOCAL_ROOT: &str = ".yolo-catalog-coordination";
const SENTINEL: &str = "sentinel";
const SENTINEL_BYTES: &[u8] = b"foreign directory remains unchanged";
const PARENT_MODE: u32 = 0o750;
const PRIVATE_MODE: u32 = 0o700;
const MODE_MASK: u32 = 0o777;

struct Fixture {
    root: tempfile::TempDir,
    runtime: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let global = yolo_paths::coordination::private_root().unwrap();
        assert_ne!(
            fs::metadata(global.path()).unwrap().dev(),
            fs::metadata(SECOND_FILESYSTEM).unwrap().dev(),
            "this regression requires distinct coordination and catalog filesystems"
        );
        let root = tempfile::Builder::new()
            .prefix(TEMP_PREFIX)
            .tempdir_in(SECOND_FILESYSTEM)
            .unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(PARENT_MODE)).unwrap();
        let runtime = root.path().join(RUNTIME);
        PrivateDirectory::open_or_create_strict(&runtime).unwrap();
        Self { root, runtime }
    }

    fn entry(&self) -> PrivateFileEntry {
        PrivateFileEntry::from_directory(
            PrivateDirectory::open(&self.runtime).unwrap(),
            &self.runtime.join(CATALOG),
        )
        .unwrap()
    }
}

#[test]
fn catalog_on_another_filesystem_retains_writes_after_runtime_replacement() {
    let fixture = Fixture::new();
    let entry = fixture.entry();
    let (mut first, rebuilt) =
        Catalog::open_rebuildable_retained(entry.reopen().unwrap(), || Ok(Vec::new())).unwrap();
    assert!(!rebuilt);
    let generation = first.bump_controller_generation().unwrap();
    let alias_root = fixture.root.path().join(LOCAL_ROOT);
    assert_eq!(
        fs::metadata(&alias_root).unwrap().dev(),
        fs::metadata(&fixture.runtime).unwrap().dev()
    );
    assert_eq!(
        fs::metadata(&alias_root).unwrap().mode() & MODE_MASK,
        PRIVATE_MODE
    );
    assert_eq!(
        fs::metadata(fixture.root.path()).unwrap().mode() & MODE_MASK,
        PARENT_MODE
    );
    let (mut second, _) = Catalog::open_rebuildable_retained(entry, || Ok(Vec::new())).unwrap();

    let displaced = fixture.root.path().join(DISPLACED_RUNTIME);
    fs::rename(&fixture.runtime, &displaced).unwrap();
    PrivateDirectory::open_or_create_strict(&fixture.runtime).unwrap();
    fs::write(fixture.runtime.join(SENTINEL), SENTINEL_BYTES).unwrap();
    assert!(second.bump_controller_generation().unwrap() > generation);
    first.integrity_check().unwrap();
    second.integrity_check().unwrap();
    assert_eq!(
        fs::read(fixture.runtime.join(SENTINEL)).unwrap(),
        SENTINEL_BYTES
    );
    assert_eq!(fs::read_dir(&fixture.runtime).unwrap().count(), 1);
    drop(first);
    drop(second);

    let reopened = Catalog::open(&displaced.join(CATALOG)).unwrap();
    assert!(reopened.controller_generation().unwrap() > generation);
    reopened.integrity_check().unwrap();
}

#[test]
fn foreign_alias_root_symlink_is_rejected_without_modifying_its_target() {
    let fixture = Fixture::new();
    let foreign = tempfile::tempdir_in(SECOND_FILESYSTEM).unwrap();
    fs::write(foreign.path().join(SENTINEL), SENTINEL_BYTES).unwrap();
    symlink(foreign.path(), fixture.root.path().join(LOCAL_ROOT)).unwrap();
    assert!(Catalog::open_rebuildable_retained(fixture.entry(), || Ok(Vec::new())).is_err());
    assert_eq!(fs::read_dir(foreign.path()).unwrap().count(), 1);
    assert_eq!(
        fs::read(foreign.path().join(SENTINEL)).unwrap(),
        SENTINEL_BYTES
    );
}

#[test]
fn existing_nonprivate_alias_root_is_not_chmodded_or_used() {
    let fixture = Fixture::new();
    let alias_root = fixture.root.path().join(LOCAL_ROOT);
    fs::create_dir(&alias_root).unwrap();
    fs::set_permissions(&alias_root, fs::Permissions::from_mode(PARENT_MODE)).unwrap();
    assert!(Catalog::open_rebuildable_retained(fixture.entry(), || Ok(Vec::new())).is_err());
    assert_eq!(
        fs::metadata(alias_root).unwrap().mode() & MODE_MASK,
        PARENT_MODE
    );
}

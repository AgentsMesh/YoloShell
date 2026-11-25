use super::catalog_alias_lease::{CatalogAliasLease, CatalogAliasShard};
use super::catalog_alias_scanner::CatalogAliasScanner;
use super::secure_vfs_alias_cleanup::CleanupIoPoint;
use crate::paths::directory::PrivateDirectory;
use std::os::unix::fs::{DirBuilderExt, symlink};
use std::path::{Path, PathBuf};
use yolo_maintenance_contract::{MaintenanceScanner, ScanChunk};

#[path = "../test_support/catalog_alias_maintenance.rs"]
mod support;
use support::*;

const MAX_SCAN_CHUNKS: usize = 4_096;
const ALIAS_NAME: &str = ".yolo-sqlite-vfs-00000000-0000-0000-0000-000000000001";
const CLEANUP_SHARD_LABEL: &str = "cleanup";
const SENTINEL: &str = "sentinel";
const SENTINEL_BYTES: &[u8] = b"replacement";
const MOVED: &str = "moved";
const PRIVATE_DIRECTORY_MODE: u32 = 0o700;

struct Fixture {
    _temp: tempfile::TempDir,
    root: PrivateDirectory,
    aliases: PrivateDirectory,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = PrivateDirectory::ensure(&temp.path().join("coordination")).unwrap();
        let aliases = crate::coordination::catalog_alias_root(&root).unwrap();
        Self {
            _temp: temp,
            root,
            aliases,
        }
    }

    fn shard(&self, label: &str) -> PrivateDirectory {
        let catalog = self._temp.path().join(label).join("catalog.sqlite");
        let name = crate::coordination::catalog_alias_shard_name(&catalog).unwrap();
        self.aliases
            .ensure_descendant(&self.aliases.path().join(name))
            .unwrap()
    }

    fn scanner(&self) -> CatalogAliasScanner {
        CatalogAliasScanner::from_roots(
            self.root.try_clone().unwrap(),
            self.aliases.try_clone().unwrap(),
        )
    }
}

#[test]
fn registered_empty_shard_is_preserved_by_shared_lease() {
    let fixture = Fixture::new();
    let shard = fixture.shard("live");
    let lease = CatalogAliasLease::try_shared(&shard).unwrap().unwrap();
    run_pass(&mut fixture.scanner());
    assert!(shard.path().is_dir());
    drop(lease);
}

#[test]
fn stale_empty_shard_is_reclaimed_and_unregistered_root_alias_is_preserved() {
    let fixture = Fixture::new();
    let shard = fixture.shard("stale");
    drop(CatalogAliasLease::try_exclusive(&shard).unwrap().unwrap());
    let unregistered = fixture.aliases.path().join(ALIAS_NAME);
    private_directory(&unregistered);
    let opened = super::secure_vfs_alias_directory::open_verified(&unregistered).unwrap();
    drop(super::secure_vfs_alias_io::create_lock_file(&opened, &unregistered).unwrap());

    run_pass(&mut fixture.scanner());

    assert!(!shard.path().exists());
    assert!(unregistered.is_dir());
    assert!(
        super::secure_vfs_alias_io::open_lock_file(&opened, &unregistered)
            .unwrap()
            .is_some()
    );
}

#[test]
fn unsafe_shard_cannot_starve_a_later_healthy_shard() {
    let fixture = Fixture::new();
    let outside = fixture._temp.path().join("outside");
    private_directory(&outside);
    let unsafe_path = shard_path(&fixture, "unsafe");
    symlink(&outside, &unsafe_path).unwrap();
    let healthy = fixture.shard("healthy");
    drop(CatalogAliasLease::try_exclusive(&healthy).unwrap().unwrap());

    run_pass(&mut fixture.scanner());

    assert!(
        unsafe_path
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(!healthy.path().exists());
}

#[test]
fn shard_replacement_at_remove_boundary_is_preserved() {
    let fixture = Fixture::new();
    let shard = fixture.shard("replace");
    drop(CatalogAliasLease::try_exclusive(&shard).unwrap().unwrap());
    let expected = shard.path().to_path_buf();
    let moved = fixture.aliases.path().join(MOVED);
    super::catalog_alias_maintenance_hooks::set_remove_hook(move |path| {
        assert_eq!(path, expected);
        std::fs::rename(path, &moved).unwrap();
        private_directory(path);
        std::fs::write(path.join(SENTINEL), SENTINEL_BYTES).unwrap();
    });

    run_pass(&mut fixture.scanner());

    assert_eq!(
        std::fs::read(shard.path().join(SENTINEL)).unwrap(),
        SENTINEL_BYTES
    );
}

#[test]
fn coordination_root_swap_cannot_redirect_shard_registration() {
    let fixture = Fixture::new();
    let catalog = fixture._temp.path().join("home/catalog.sqlite");
    let root_path = fixture.root.path().to_path_buf();
    let moved = fixture._temp.path().join(MOVED);
    super::catalog_alias_maintenance_hooks::set_retain_hook(move |path| {
        assert_eq!(path, root_path);
        std::fs::rename(path, &moved).unwrap();
        let replacement = PrivateDirectory::ensure(path).unwrap();
        std::fs::write(replacement.path().join(SENTINEL), SENTINEL_BYTES).unwrap();
    });

    assert!(
        CatalogAliasShard::retain_for_test(&catalog, fixture.root.try_clone().unwrap()).is_err()
    );
    assert_eq!(
        std::fs::read(fixture.root.path().join(SENTINEL)).unwrap(),
        SENTINEL_BYTES
    );
}

#[test]
fn cleanup_unlink_and_sync_io_failures_reach_scanner_backoff_boundary() {
    for point in [
        CleanupIoPoint::EntryRemove,
        CleanupIoPoint::DirectoryRemove,
        CleanupIoPoint::ParentSync,
    ] {
        let fixture = Fixture::new();
        let shard = fixture.shard(CLEANUP_SHARD_LABEL);
        drop(CatalogAliasLease::try_exclusive(&shard).unwrap().unwrap());
        let alias = shard.path().join(ALIAS_NAME);
        private_directory(&alias);
        let opened = super::secure_vfs_alias_directory::open_verified(&alias).unwrap();
        drop(super::secure_vfs_alias_io::create_lock_file(&opened, &alias).unwrap());
        super::catalog_alias_maintenance_hooks::set_cleanup_io_hook(point);

        let error = run_until_error(&mut fixture.scanner());

        assert!(error.chain().any(|cause| {
            cause
                .downcast_ref::<std::io::Error>()
                .is_some_and(|error| error.raw_os_error() == Some(libc::EIO))
        }));
    }
}

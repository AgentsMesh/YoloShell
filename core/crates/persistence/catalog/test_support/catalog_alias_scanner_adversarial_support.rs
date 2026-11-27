use super::catalog_alias_lease::{CatalogAliasLease, CatalogAliasShard};
use super::catalog_alias_scanner::{CatalogAliasScanner, ENTRY_BUDGET};
use crate::paths::directory::PrivateDirectory;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use yolo_maintenance_contract::{MaintenanceScanner, ScanChunk};

const MAX_SCANNER_CHUNKS: usize = 4_096;
const FIRST_ALIAS_ID: u128 = 1;
const EXTRA_ALIAS_COUNT: usize = 2;
const BUSY_ALIAS_ID: u128 = 1;
const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
const UNKNOWN_SHARD_ENTRY: &str = "unknown-entry";
const UNKNOWN_SHARD_BYTES: &[u8] = b"preserve shard";
const CONTENT_SHARD: &str = "new-content";
const LEASE_SHARD: &str = "new-lease";
const DISAPPEARED_SHARD: &str = "disappeared";
const COORDINATION_DIRECTORY: &str = "coordination";
const CATALOG_DATABASE_NAME: &str = "catalog.sqlite";
const SCAN_INCOMPLETE_DETAIL: &str = "catalog alias scanner did not complete a pass";

struct Fixture {
    _temp: tempfile::TempDir,
    root: PrivateDirectory,
    aliases: PrivateDirectory,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = PrivateDirectory::ensure(&temp.path().join(COORDINATION_DIRECTORY)).unwrap();
        let aliases = crate::coordination::catalog_alias_root(&root).unwrap();
        Self {
            _temp: temp,
            root,
            aliases,
        }
    }

    fn shard(&self, label: &str) -> PrivateDirectory {
        let path = self.shard_path(label);
        self.aliases.ensure_descendant(&path).unwrap()
    }

    fn shard_path(&self, label: &str) -> PathBuf {
        let catalog = self._temp.path().join(label).join(CATALOG_DATABASE_NAME);
        let name = crate::coordination::catalog_alias_shard_name(&catalog).unwrap();
        self.aliases.path().join(name)
    }

    fn scanner(&self) -> CatalogAliasScanner {
        CatalogAliasScanner::from_roots(
            self.root.try_clone().unwrap(),
            self.aliases.try_clone().unwrap(),
        )
    }
}

fn create_unlocked_alias(path: &Path) {
    private_directory(path);
    let opened = super::secure_vfs_alias_directory::open_verified(path).unwrap();
    drop(super::secure_vfs_alias_io::create_lock_file(&opened, path).unwrap());
}

fn run_pass(scanner: &mut CatalogAliasScanner) {
    for _ in 0..MAX_SCANNER_CHUNKS {
        match scanner.scan_chunk().unwrap() {
            ScanChunk::PassComplete => return,
            ScanChunk::Busy => std::thread::yield_now(),
            ScanChunk::Progress => {}
        }
    }
    panic!("{SCAN_INCOMPLETE_DETAIL}");
}

fn private_directory(path: &Path) {
    std::fs::DirBuilder::new()
        .mode(PRIVATE_DIRECTORY_MODE)
        .create(path)
        .unwrap();
}

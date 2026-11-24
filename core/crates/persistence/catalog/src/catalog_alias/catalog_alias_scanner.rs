use super::catalog_alias_lease::{CatalogAliasLease, SHARD_LEASE_FILE};
use super::catalog_alias_shard_scan::{ShardChunk, ShardScan};
use super::catalog_alias_stream::AliasDirectoryStream;
use crate::paths::directory::{DirectoryRevision, PrivateDirectory};
use anyhow::{Context, Result};
use std::ffi::{CString, OsStr};
use std::os::unix::ffi::OsStrExt;
use yolo_maintenance_contract::{MaintenanceScanner, ScanChunk};

const SCANNER_LABEL: &str = "catalog-aliases";
pub(super) const ENTRY_BUDGET: usize = 16;
const REMOVE_CONTEXT: &str = "remove empty catalog alias shard";

pub struct CatalogAliasScanner {
    root: Option<PrivateDirectory>,
    aliases: Option<PrivateDirectory>,
    stream: Option<AliasDirectoryStream>,
    revision: Option<DirectoryRevision>,
    shard: Option<ShardScan>,
}

impl CatalogAliasScanner {
    pub fn new() -> Self {
        Self {
            root: None,
            aliases: None,
            stream: None,
            revision: None,
            shard: None,
        }
    }

    #[cfg(test)]
    pub(super) fn from_roots(root: PrivateDirectory, aliases: PrivateDirectory) -> Self {
        Self {
            root: Some(root),
            aliases: Some(aliases),
            stream: None,
            revision: None,
            shard: None,
        }
    }

    fn scan(&mut self) -> Result<ScanChunk> {
        if self.shard.is_some() {
            return self.scan_shard();
        }
        if self.root.is_none() {
            self.root = Some(crate::coordination::private_root()?);
        }
        let root = self.root.as_ref().expect("root initialized");
        let Some(_lifecycle) = crate::coordination::try_lifecycle(root)? else {
            return Ok(ScanChunk::Busy);
        };
        if self.aliases.is_none() {
            self.aliases = Some(crate::coordination::catalog_alias_root(root)?);
        }
        self.scan_root_locked()
    }

    fn scan_root_locked(&mut self) -> Result<ScanChunk> {
        let aliases = self.aliases.as_ref().expect("alias root initialized");
        aliases.verify_path()?;
        let revision = aliases.revision()?;
        if self.revision != Some(revision) || self.stream.is_none() {
            self.stream = Some(AliasDirectoryStream::open(aliases.descriptor())?);
            self.revision = Some(revision);
        }
        for _ in 0..ENTRY_BUDGET {
            let Some(name) = self
                .stream
                .as_mut()
                .expect("stream initialized")
                .next_name()?
            else {
                self.reset_pass();
                return Ok(ScanChunk::PassComplete);
            };
            if crate::coordination::is_catalog_alias_digest(&name) {
                if let Some(shard) = ShardScan::open(aliases, name)? {
                    self.shard = Some(shard);
                    return Ok(ScanChunk::Progress);
                }
            }
        }
        Ok(ScanChunk::Progress)
    }

    fn scan_shard(&mut self) -> Result<ScanChunk> {
        let outcome = self
            .shard
            .as_mut()
            .expect("shard initialized")
            .scan_chunk()?;
        if matches!(outcome, ShardChunk::Progress) {
            return Ok(ScanChunk::Progress);
        }
        let shard = self.shard.take().expect("shard initialized");
        if shard.blocked {
            return Ok(ScanChunk::Progress);
        }
        self.remove_empty_shard(shard)
    }

    fn remove_empty_shard(&mut self, shard: ShardScan) -> Result<ScanChunk> {
        let (name, directory) = shard.finish();
        let root = self.root.as_ref().expect("root initialized");
        let Some(_lifecycle) = crate::coordination::try_lifecycle(root)? else {
            return Ok(ScanChunk::Progress);
        };
        let aliases = self.aliases.as_ref().expect("alias root initialized");
        let path = aliases.path().join(&name);
        run_remove_test_hook(&path);
        let reopened = match aliases.open_descendant(&path) {
            Ok(directory) => directory,
            Err(error) if super::secure_vfs_cleanup_error::preserve(&error) => {
                return Ok(ScanChunk::Progress);
            }
            Err(error) => return Err(error),
        };
        if !reopened.same_identity(&directory) {
            return Ok(ScanChunk::Progress);
        }
        let Some(lease) = CatalogAliasLease::try_exclusive(&reopened)? else {
            return Ok(ScanChunk::Progress);
        };
        if !contains_only_lease(&reopened)? {
            return Ok(ScanChunk::Progress);
        }
        lease.unlink_owned(&reopened)?;
        remove_directory(aliases, &name)?;
        self.reset_pass();
        Ok(ScanChunk::Progress)
    }

    fn reset_pass(&mut self) {
        self.stream = None;
        self.revision = None;
    }
}

impl MaintenanceScanner for CatalogAliasScanner {
    fn label(&self) -> &'static str {
        SCANNER_LABEL
    }

    fn scan_chunk(&mut self) -> Result<ScanChunk> {
        let result = self.scan();
        if result.is_err() {
            self.reset();
        }
        result
    }

    fn reset(&mut self) {
        self.shard = None;
        self.stream = None;
        self.revision = None;
        self.aliases = None;
        self.root = None;
    }
}

fn contains_only_lease(directory: &PrivateDirectory) -> Result<bool> {
    let mut stream = AliasDirectoryStream::open(directory.descriptor())?;
    while let Some(name) = stream.next_name()? {
        if name != OsStr::new(SHARD_LEASE_FILE) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn remove_directory(parent: &PrivateDirectory, name: &OsStr) -> Result<()> {
    let name = CString::new(name.as_bytes())?;
    let result = unsafe { libc::unlinkat(parent.descriptor(), name.as_ptr(), libc::AT_REMOVEDIR) };
    if result == 0 {
        return parent.sync();
    }
    let error = std::io::Error::last_os_error();
    if super::secure_vfs_cleanup_error::preserve_io(&error) {
        return Ok(());
    }
    Err(error).context(REMOVE_CONTEXT)
}

fn run_remove_test_hook(path: &std::path::Path) {
    #[cfg(all(test, feature = "yolo-unit-tests"))]
    super::catalog_alias_maintenance_hooks::run_remove_hook(path);
    #[cfg(not(all(test, feature = "yolo-unit-tests")))]
    let _ = path;
}

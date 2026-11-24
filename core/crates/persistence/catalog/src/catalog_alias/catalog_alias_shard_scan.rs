use super::catalog_alias_lease::{CatalogAliasLease, SHARD_LEASE_FILE};
use super::catalog_alias_stream::AliasDirectoryStream;
use crate::paths::directory::PrivateDirectory;
use anyhow::Result;
use std::ffi::{OsStr, OsString};

const ENTRY_BUDGET: usize = 16;

pub(super) struct ShardScan {
    name: OsString,
    directory: PrivateDirectory,
    _lease: CatalogAliasLease,
    stream: AliasDirectoryStream,
    pub(super) blocked: bool,
}

pub(super) enum ShardChunk {
    Progress,
    Complete,
}

impl ShardScan {
    pub(super) fn open(parent: &PrivateDirectory, name: OsString) -> Result<Option<Self>> {
        let path = parent.path().join(&name);
        let directory = match parent.open_descendant(&path) {
            Ok(directory) => directory,
            Err(error) if super::secure_vfs_cleanup_error::preserve(&error) => return Ok(None),
            Err(error) => return Err(error),
        };
        let lease = match CatalogAliasLease::try_exclusive(&directory) {
            Ok(Some(lease)) => lease,
            Ok(None) => return Ok(None),
            Err(error) if super::secure_vfs_cleanup_error::preserve(&error) => return Ok(None),
            Err(error) => return Err(error),
        };
        directory.verify_path()?;
        let stream = AliasDirectoryStream::open(directory.descriptor())?;
        Ok(Some(Self {
            name,
            directory,
            _lease: lease,
            stream,
            blocked: false,
        }))
    }

    pub(super) fn scan_chunk(&mut self) -> Result<ShardChunk> {
        self.directory.verify_path()?;
        for _ in 0..ENTRY_BUDGET {
            let Some(name) = self.stream.next_name()? else {
                return Ok(ShardChunk::Complete);
            };
            if name == OsStr::new(SHARD_LEASE_FILE) {
                continue;
            }
            if super::secure_vfs_cleanup::is_alias_candidate(&name) {
                if super::secure_vfs_cleanup::cleanup_named_candidate(&self.directory, &name)? {
                    self.stream = AliasDirectoryStream::open(self.directory.descriptor())?;
                    self.blocked = false;
                } else {
                    self.blocked = true;
                }
                return Ok(ShardChunk::Progress);
            }
            self.blocked = true;
        }
        Ok(ShardChunk::Progress)
    }

    pub(super) fn finish(self) -> (OsString, PrivateDirectory) {
        (self.name, self.directory)
    }
}

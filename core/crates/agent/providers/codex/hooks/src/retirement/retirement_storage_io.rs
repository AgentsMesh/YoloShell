use super::CodexRetirementStorage;
use crate::codex_hooks::retirement_constants::{CORRUPT, STORAGE};
use crate::paths::identity::FileIdentity;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::{Context, Result, ensure};
use serde::{Serialize, de::DeserializeOwned};
use std::path::Path;

const MAX_BYTES: u64 = crate::paths::MAX_DURABLE_JSON_BYTES;

impl CodexRetirementStorage {
    pub(super) fn publish<T>(
        &self,
        path: &Path,
        expected: &T,
        commit_fence: impl FnMut() -> Result<()>,
    ) -> Result<(T, FileIdentity, bool)>
    where
        T: Clone + DeserializeOwned + Eq + Serialize,
    {
        let encoded = serde_json::to_vec_pretty(expected).context(STORAGE)?;
        ensure!(u64::try_from(encoded.len())? <= MAX_BYTES, CORRUPT);
        let entry = self.entry(path)?;
        match entry.create_bytes_exclusive_with_fence(&encoded, MAX_BYTES, commit_fence) {
            Ok(identity) => self.finish_publish(path, expected, identity, false),
            Err(cause) => match self.read(path) {
                Ok(Some((record, identity))) if &record == expected => Ok((record, identity, true)),
                _ => Err(cause.context(STORAGE)),
            },
        }
    }

    fn finish_publish<T: Clone + DeserializeOwned + Eq>(
        &self,
        path: &Path,
        expected: &T,
        identity: FileIdentity,
        replay: bool,
    ) -> Result<(T, FileIdentity, bool)> {
        let (record, current) = self.read(path)?.context(STORAGE)?;
        ensure!(&record == expected && current == identity, CORRUPT);
        Ok((record, current, replay))
    }

    pub(super) fn inspect(&self, path: &Path) -> Result<Option<FileIdentity>> {
        self.entry(path)?.inspect(MAX_BYTES).context(STORAGE)
    }

    pub(super) fn read<T: DeserializeOwned>(
        &self,
        path: &Path,
    ) -> Result<Option<(T, FileIdentity)>> {
        let entry = self.entry(path)?;
        if entry.is_missing()? {
            self.directory.verify_path().context(STORAGE)?;
            return Ok(None);
        }
        crate::paths::read_entry_guarded_with_identity(entry, || self.directory.verify_path())
            .map(Some)
            .context(STORAGE)
    }

    fn entry(&self, path: &Path) -> Result<PrivateFileEntry> {
        PrivateFileEntry::from_directory(self.directory.try_clone()?, path).context(STORAGE)
    }
}

use super::constants::MAX_DURABLE_JSON_BYTES;
use super::durable_json_read::read_retained_entry_guarded_with_opened;
use super::identity::FileIdentity;
use super::private_file::PrivateFileEntry;
use super::process_file_lock::ProcessFileLock;
use anyhow::Result;
use serde::de::DeserializeOwned;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct RetainedJsonFileObservation {
    entry: Arc<PrivateFileEntry>,
    identity: FileIdentity,
    _file: Arc<std::fs::File>,
    _lock: Option<Arc<ProcessFileLock>>,
}

impl RetainedJsonFileObservation {
    pub fn bind<T: DeserializeOwned>(path: &Path) -> Result<(T, Self)> {
        Self::read(Arc::new(PrivateFileEntry::open(path)?))
    }

    pub fn refresh<T: DeserializeOwned>(&self) -> Result<(T, Self)> {
        Self::read(Arc::clone(&self.entry))
    }

    pub fn verify_unchanged(&self) -> Result<()> {
        self.identity.verify_file_bounded(
            self._file.as_ref(),
            self.identity.length(),
            self.entry.path(),
            MAX_DURABLE_JSON_BYTES,
        )?;
        self.entry
            .verify_retained_path_unchanged(self.identity, MAX_DURABLE_JSON_BYTES)
            .map(|_| ())
    }

    pub fn identity(&self) -> FileIdentity {
        self.identity
    }

    pub fn read_entry_guarded_with_shared_lock<T, F>(
        entry: Arc<PrivateFileEntry>,
        guard: F,
        timeout: Duration,
        retry_delay: Duration,
    ) -> Result<(T, Self)>
    where
        T: DeserializeOwned,
        F: FnOnce() -> Result<()>,
    {
        let (value, opened) = read_retained_entry_guarded_with_opened(entry.as_ref(), guard)?;
        let lock = entry.lock_opened_retained_shared_until(
            &opened,
            MAX_DURABLE_JSON_BYTES,
            timeout,
            retry_delay,
        )?;
        let identity = opened.identity();
        let (file, _) = opened.into_parts();
        Ok((
            value,
            Self {
                entry,
                identity,
                _file: Arc::new(file),
                _lock: Some(Arc::new(lock)),
            },
        ))
    }

    fn read<T: DeserializeOwned>(entry: Arc<PrivateFileEntry>) -> Result<(T, Self)> {
        let (value, opened) = read_retained_entry_guarded_with_opened(entry.as_ref(), || Ok(()))?;
        let identity = opened.identity();
        let (file, _) = opened.into_parts();
        Ok((
            value,
            Self {
                entry,
                identity,
                _file: Arc::new(file),
                _lock: None,
            },
        ))
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/retained_json_observation_tests.rs"]
mod tests;

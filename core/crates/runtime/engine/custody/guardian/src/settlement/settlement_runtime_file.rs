use crate::model::RuntimeRecord;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::constants::MAX_DURABLE_JSON_BYTES;
use crate::paths::identity::FileIdentity;
use anyhow::{Result, ensure};
use std::time::Duration;

const RUNTIME_FILE_DETAIL: &str = "detached guardian retained runtime file changed unexpectedly";
const RUNTIME_RETAIN_LOCK_TIMEOUT: Duration = Duration::ZERO;

#[derive(Debug)]
pub(super) struct RetainedRuntimeFile {
    _process_lock: crate::paths::process_file_lock::ProcessFileLock,
    _file: std::fs::File,
}

impl RetainedRuntimeFile {
    pub(super) fn capture(
        namespace: &NamespaceLease,
        paths: &Paths,
        runtime: &RuntimeRecord,
        identity: FileIdentity,
    ) -> Result<Self> {
        let path = paths.runtime_record(runtime.terminal_id);
        let (opened, process_lock) = namespace
            .file(&path)?
            .open_retained_with_shared_lock_until(
                MAX_DURABLE_JSON_BYTES,
                RUNTIME_RETAIN_LOCK_TIMEOUT,
                super::settlement::POLL_INTERVAL,
            )?;
        ensure!(opened.identity() == identity, RUNTIME_FILE_DETAIL);
        let (file, _) = opened.into_parts();
        Ok(Self {
            _process_lock: process_lock,
            _file: file,
        })
    }
}

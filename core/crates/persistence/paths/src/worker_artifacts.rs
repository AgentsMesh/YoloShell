use super::Paths;
use anyhow::Result;
use serde::Serialize;
use std::path::PathBuf;
use uuid::Uuid;

const STARTUP_CHECKPOINT_PATH_ERROR: &str = "worker startup checkpoint path mismatch";
const STARTUP_CHECKPOINT_SIZE_ERROR: &str = "worker startup checkpoint exceeds its byte limit";

impl Paths {
    pub fn worker_startup_checkpoint(&self, terminal_id: Uuid) -> PathBuf {
        self.session_dir(terminal_id)
            .join(super::constants::WORKER_STARTUP_CHECKPOINT_FILE_NAME)
    }
}

pub fn validate_worker_startup_checkpoint_value<T: Serialize>(
    entry: &super::private_file::PrivateFileEntry,
    value: &T,
) -> Result<()> {
    anyhow::ensure!(
        entry.path().file_name()
            == Some(std::ffi::OsStr::new(
                super::constants::WORKER_STARTUP_CHECKPOINT_FILE_NAME,
            )),
        STARTUP_CHECKPOINT_PATH_ERROR
    );
    let encoded = serde_json::to_vec_pretty(value)?;
    anyhow::ensure!(
        u64::try_from(encoded.len())? <= super::constants::WORKER_STARTUP_CHECKPOINT_MAX_BYTES,
        STARTUP_CHECKPOINT_SIZE_ERROR
    );
    Ok(())
}

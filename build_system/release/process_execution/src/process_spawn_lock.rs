use anyhow::{Context, Result};
use std::sync::{Mutex, MutexGuard};

static PROCESS_SPAWN_LOCK: Mutex<()> = Mutex::new(());

pub(crate) fn acquire() -> Result<MutexGuard<'static, ()>> {
    PROCESS_SPAWN_LOCK
        .lock()
        .map_err(|error| anyhow::anyhow!(error.to_string()))
        .context("acquire process spawn lock")
}

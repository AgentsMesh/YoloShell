use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn current_unix_seconds() -> Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| anyhow::anyhow!(crate::constants::CLOCK_ERROR))
}

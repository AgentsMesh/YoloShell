use crate::constants::INVALID_CLOCK;
use anyhow::{Context, Result};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn now_ms() -> Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context(INVALID_CLOCK)?
        .as_millis()
        .try_into()
        .context(INVALID_CLOCK)
}

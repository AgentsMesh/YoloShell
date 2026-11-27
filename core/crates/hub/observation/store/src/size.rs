use crate::constants::LIMIT;
use anyhow::Result;
use serde::Serialize;
use std::io::{self, Write};
use yolo_paths::constants::MAX_DURABLE_JSON_BYTES;

/// Count the exact durable serializer output before its allocating write path.
pub(crate) fn validate(value: &impl Serialize) -> Result<()> {
    serde_json::to_writer_pretty(
        Counter {
            remaining: MAX_DURABLE_JSON_BYTES,
        },
        value,
    )?;
    Ok(())
}
pub(crate) struct Counter {
    pub remaining: u64,
}
impl Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let count = u64::try_from(bytes.len()).map_err(|_| io::Error::other(LIMIT))?;
        self.remaining = self
            .remaining
            .checked_sub(count)
            .ok_or_else(|| io::Error::other(LIMIT))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

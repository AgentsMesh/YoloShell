use crate::fake_claude_reference_types::{DurableAuthority, ENDPOINT_SCHEMA_VERSION};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;

const TEMPORARY_EXTENSION: &str = "next";
const MAX_STATE_BYTES: u64 = 1_048_576;
const SIZE_LIMIT_PROBE_BYTES: u64 = 1;
const STATE_TOO_LARGE: &str = "fake Claude authority exceeds its size limit";
const STATE_SCHEMA_INVALID: &str = "fake Claude authority schema is unsupported";

pub(crate) fn load(path: &Path) -> io::Result<Option<DurableAuthority>> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_STATE_BYTES + SIZE_LIMIT_PROBE_BYTES)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_STATE_BYTES {
        return Err(io::Error::other(STATE_TOO_LARGE));
    }
    let value: DurableAuthority = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
    if value.schema_version != ENDPOINT_SCHEMA_VERSION {
        return Err(io::Error::other(STATE_SCHEMA_INVALID));
    }
    Ok(Some(value))
}

pub(crate) fn save(path: &Path, value: &DurableAuthority) -> io::Result<()> {
    let temporary = path.with_extension(TEMPORARY_EXTENSION);
    let bytes = serde_json::to_vec(value).map_err(io::Error::other)?;
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temporary)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    std::fs::rename(&temporary, path)?;
    if let Some(parent) = path.parent() {
        File::open(parent)?.sync_all()?;
    }
    Ok(())
}

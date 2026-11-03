use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;

const LOST_MARKER_SCHEMA_VERSION: u32 = 1;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LostMarker {
    schema_version: u32,
    expected: crate::paths::identity::FileIdentity,
}

pub fn create_lost_marker(
    path: &Path,
    expected: crate::paths::identity::FileIdentity,
) -> Result<crate::paths::identity::FileIdentity> {
    let bytes = serde_json::to_vec(&LostMarker {
        schema_version: LOST_MARKER_SCHEMA_VERSION,
        expected,
    })?;
    crate::paths::private_file::PrivateFileEntry::open(path)?
        .create_bytes_exclusive(&bytes, crate::claude_hooks::MAX_CLAUDE_SETTINGS_BYTES)
}

pub fn lost_marker_matches(
    path: &Path,
    identity: crate::paths::identity::FileIdentity,
    expected: crate::paths::identity::FileIdentity,
) -> Result<bool> {
    let entry = crate::paths::private_file::PrivateFileEntry::open(path)?;
    let mut opened = entry.open_read(crate::claude_hooks::MAX_CLAUDE_SETTINGS_BYTES)?;
    ensure!(
        opened.identity().unchanged(identity),
        "failure marker changed"
    );
    let capacity = usize::try_from(identity.length()).context("failure marker length overflow")?;
    let mut bytes = Vec::with_capacity(capacity);
    opened.file_mut().read_to_end(&mut bytes)?;
    let current = entry
        .inspect(crate::claude_hooks::MAX_CLAUDE_SETTINGS_BYTES)?
        .context("failure marker disappeared")?;
    ensure!(current.unchanged(identity), "failure marker changed");
    let Ok(marker) = serde_json::from_slice::<LostMarker>(&bytes) else {
        return Ok(false);
    };
    Ok(marker.schema_version == LOST_MARKER_SCHEMA_VERSION && marker.expected == expected)
}

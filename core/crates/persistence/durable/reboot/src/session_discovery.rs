use super::constants::*;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::directory_stream::DirectoryStream;
use anyhow::{Result, ensure};
use uuid::Uuid;

pub(super) fn session_ids(paths: &Paths, namespace: &NamespaceLease) -> Result<Vec<Uuid>> {
    let sessions = namespace.directory(&paths.sessions_dir())?;
    let mut stream = DirectoryStream::open(sessions.descriptor())?;
    let mut identifiers = Vec::new();
    let mut scanned_entries = 0_usize;
    while let Some(name) = stream.next_name()? {
        scanned_entries = scanned_entries
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!(SESSION_CAPACITY_EXCEEDED))?;
        ensure!(
            scanned_entries <= crate::constants::MAX_MANAGED_SESSIONS,
            SESSION_CAPACITY_EXCEEDED
        );
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(identifier) = crate::serde_uuid::parse_canonical(name) else {
            continue;
        };
        let session = paths.session_dir(identifier);
        namespace.directory(&session)?.verify_path()?;
        identifiers.push(identifier);
    }
    sessions.verify_path()?;
    namespace.verify()?;
    identifiers.sort_unstable();
    identifiers.dedup();
    Ok(identifiers)
}

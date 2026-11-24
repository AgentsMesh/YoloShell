use crate::paths::identity::FileIdentity;
use crate::pty_custody::CustodyDurableRecord;
use anyhow::{Context, Result, ensure};
use std::path::Path;
use yolo_paths::durable_mutation::SessionDurableMutationGuard;

use super::PREPARATION_DETAIL;

pub(super) fn committed(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    expected: &CustodyDurableRecord,
    committed: Option<FileIdentity>,
) -> Result<FileIdentity> {
    let committed = committed.context(PREPARATION_DETAIL)?;
    require_exact(guard, path, expected, committed)?;
    let parent = path.parent().context(PREPARATION_DETAIL)?;
    guard.namespace().directory(parent)?.sync()?;
    require_exact(guard, path, expected, committed)?;
    Ok(committed)
}

fn require_exact(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    expected: &CustodyDurableRecord,
    identity: FileIdentity,
) -> Result<()> {
    let current: (CustodyDurableRecord, FileIdentity) =
        guard.namespace().read_json_with_identity(path)?;
    ensure!(current == (expected.clone(), identity), PREPARATION_DETAIL);
    Ok(())
}

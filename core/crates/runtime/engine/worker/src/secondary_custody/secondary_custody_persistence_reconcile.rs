use super::{CustodyDurableRecord, FileIdentity, RUNTIME_IDENTITY_DETAIL, WorkerNamespace};
use crate::model::RuntimeRecord;
use crate::namespace_lease::NamespaceLease;
use anyhow::{Context, Result, ensure};
use std::path::Path;

pub(crate) fn committed(
    namespace: &WorkerNamespace,
    path: &Path,
    replacement: &CustodyDurableRecord,
    committed: Option<FileIdentity>,
    installed: Option<FileIdentity>,
    runtime_path: &Path,
    runtime: &RuntimeRecord,
    runtime_identity: FileIdentity,
) -> Result<FileIdentity> {
    let committed = committed.context(RUNTIME_IDENTITY_DETAIL)?;
    let (current, current_identity): (CustodyDurableRecord, FileIdentity) =
        namespace.read_json_with_identity(path)?;
    // Atomic rename can change ctime between the commit observer and final installation.
    ensure!(
        current == *replacement
            && current_identity.same_content_version(committed)
            && installed.is_none_or(|identity| identity == current_identity),
        RUNTIME_IDENTITY_DETAIL
    );
    let installed_identity = current_identity;
    require_exact(
        namespace,
        path,
        replacement,
        installed_identity,
        runtime_path,
        runtime,
        runtime_identity,
    )?;
    let parent = path.parent().context(RUNTIME_IDENTITY_DETAIL)?;
    namespace.lease().directory(parent)?.sync()?;
    require_exact(
        namespace,
        path,
        replacement,
        installed_identity,
        runtime_path,
        runtime,
        runtime_identity,
    )?;
    Ok(installed_identity)
}

pub(crate) fn require_runtime_exact(
    namespace: &NamespaceLease,
    path: &Path,
    expected: &RuntimeRecord,
    identity: FileIdentity,
) -> Result<()> {
    let current: (RuntimeRecord, FileIdentity) = namespace.read_json_with_identity(path)?;
    ensure!(
        current == (expected.clone(), identity),
        RUNTIME_IDENTITY_DETAIL
    );
    Ok(())
}

fn require_exact(
    namespace: &WorkerNamespace,
    path: &Path,
    replacement: &CustodyDurableRecord,
    committed: FileIdentity,
    runtime_path: &Path,
    runtime: &RuntimeRecord,
    runtime_identity: FileIdentity,
) -> Result<()> {
    let current: (CustodyDurableRecord, FileIdentity) = namespace.read_json_with_identity(path)?;
    let current_runtime: (RuntimeRecord, FileIdentity) =
        namespace.read_json_with_identity(runtime_path)?;
    ensure!(
        current == (replacement.clone(), committed)
            && current_runtime == (runtime.clone(), runtime_identity),
        RUNTIME_IDENTITY_DETAIL
    );
    Ok(())
}

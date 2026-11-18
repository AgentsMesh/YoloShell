use super::super::{Paths, WorkerNamespace, startup_checkpoint};
use super::FENCE_ERROR;
use crate::model::RuntimeRecord;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::CustodyDurableRecord;
use anyhow::Result;
use yolo_paths::durable_mutation::DurableMutationFencePhase;

pub(crate) enum CustodyState {
    Missing,
    Source {
        record: CustodyDurableRecord,
        identity: FileIdentity,
    },
    Installed(CustodyDurableRecord, FileIdentity),
}

pub(crate) fn capture(
    namespace: &WorkerNamespace,
    paths: &Paths,
    checkpoint: &startup_checkpoint::StartupCheckpointEvidence,
    pending: &CustodyDurableRecord,
) -> Result<CustodyState> {
    let path = paths.pty_custody(checkpoint.terminal_id);
    let current: Option<(CustodyDurableRecord, _, _)> = if namespace.file_is_missing(&path)? {
        None
    } else {
        Some(namespace.lease().read_json_with_versions(&path)?)
    };
    let Some((current, identity, version)) = current else {
        anyhow::ensure!(
            checkpoint.pending_pty_custody_rollover.is_none(),
            FENCE_ERROR
        );
        return Ok(CustodyState::Missing);
    };
    if current == *pending {
        return Ok(CustodyState::Installed(current, identity));
    }
    let source = checkpoint
        .pending_pty_custody_rollover
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!(FENCE_ERROR))?;
    anyhow::ensure!(
        version == source.source_version
            && crate::pty_custody::durable_record_digest(&current)? == source.source_sha256,
        FENCE_ERROR
    );
    Ok(CustodyState::Source {
        record: current,
        identity,
    })
}

pub(crate) fn require_exact(
    namespace: &WorkerNamespace,
    paths: &Paths,
    runtime: &RuntimeRecord,
    pending: &CustodyDurableRecord,
    expected: &CustodyState,
    phase: DurableMutationFencePhase,
) -> Result<()> {
    let path = paths.pty_custody(runtime.terminal_id);
    let actual: Option<(CustodyDurableRecord, _)> = (!namespace.file_is_missing(&path)?)
        .then(|| namespace.read_json_with_identity(&path))
        .transpose()?;
    if phase == DurableMutationFencePhase::Installed {
        return require_installed(namespace, &path, pending, expected, actual);
    }
    anyhow::ensure!(
        !matches!(expected, CustodyState::Installed(..)),
        FENCE_ERROR
    );
    match (expected, actual) {
        (CustodyState::Missing, None) => Ok(()),
        (
            CustodyState::Source {
                record,
                identity: expected,
            },
            Some((current, identity)),
        ) => {
            anyhow::ensure!(
                identity == *expected && current == *record && current != *pending,
                FENCE_ERROR
            );
            Ok(())
        }
        _ => anyhow::bail!(FENCE_ERROR),
    }
}

fn require_installed(
    namespace: &WorkerNamespace,
    path: &std::path::Path,
    pending: &CustodyDurableRecord,
    expected: &CustodyState,
    actual: Option<(CustodyDurableRecord, FileIdentity)>,
) -> Result<()> {
    let (current, identity) = actual.ok_or_else(|| anyhow::anyhow!(FENCE_ERROR))?;
    anyhow::ensure!(current == *pending, FENCE_ERROR);
    if let CustodyState::Installed(record, expected) = expected {
        anyhow::ensure!(current == *record && identity == *expected, FENCE_ERROR);
        return Ok(());
    }
    let confirmed: (CustodyDurableRecord, FileIdentity) =
        namespace.read_json_with_identity(path)?;
    anyhow::ensure!(confirmed == (current, identity), FENCE_ERROR);
    Ok(())
}

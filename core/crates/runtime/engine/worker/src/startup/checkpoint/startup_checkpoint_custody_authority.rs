use super::super::Paths;
use super::{
    StartupCheckpointEvidence, StartupPredecessorAuthority, StartupPredecessorCustodyAuthority,
};
use crate::model::RuntimeRecord;
use crate::pty_custody::CustodyDurableRecord;
use anyhow::Result;

pub(crate) fn capture(
    mutation: &super::mutation::CheckpointMutation<'_>,
    paths: &Paths,
    stored: &StartupCheckpointEvidence,
    successor: super::StartupLaunchFence,
) -> Result<(
    Option<StartupPredecessorAuthority>,
    Option<super::StartupPtyCustodyRollover>,
)> {
    let Some(runtime_sha256) = stored.predecessor_runtime_sha256 else {
        anyhow::ensure!(stored.pending_pty_custody_rollover.is_none(), error());
        return Ok((None, None));
    };
    let (runtime, _): (RuntimeRecord, _) = mutation
        .read_optional_managed(&paths.runtime_record(stored.terminal_id))?
        .ok_or_else(|| anyhow::anyhow!(error()))?;
    anyhow::ensure!(
        super::predecessor::runtime_digest(&runtime)? == runtime_sha256,
        error()
    );
    let custody = capture_custody(mutation, paths, stored)?;
    let authority = StartupPredecessorAuthority {
        checkpoint_identity: mutation
            .read_optional()?
            .ok_or_else(|| anyhow::anyhow!(error()))?
            .1,
        runtime_sha256,
        custody,
        successor,
    };
    Ok((Some(authority), stored.pending_pty_custody_rollover.clone()))
}

pub(crate) fn capture_custody(
    mutation: &super::mutation::CheckpointMutation<'_>,
    paths: &Paths,
    stored: &StartupCheckpointEvidence,
) -> Result<StartupPredecessorCustodyAuthority> {
    let path = paths.pty_custody(stored.terminal_id);
    let current: Option<(CustodyDurableRecord, _, _)> =
        mutation.read_optional_managed_with_versions(&path)?;
    match (stored.pending_pty_custody_rollover.as_ref(), current) {
        (None, None) => Ok(StartupPredecessorCustodyAuthority::Absent),
        (Some(rollover), Some((record, identity, version))) => {
            let sha256 = crate::pty_custody::durable_record_digest(&record)?;
            anyhow::ensure!(
                version == rollover.source_version
                    && sha256 == rollover.source_sha256
                    && record.is_valid(),
                error()
            );
            Ok(StartupPredecessorCustodyAuthority::Present { identity, sha256 })
        }
        _ => anyhow::bail!(error()),
    }
}

pub(crate) fn error() -> &'static str {
    super::constants::PREDECESSOR_CUSTODY_ERROR
}

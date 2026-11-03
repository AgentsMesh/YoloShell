use super::super::{Paths, WorkerNamespace};
use super::{StartupCheckpointEvidence, StartupPredecessorAuthority, StartupStatus};
use crate::model::RuntimeRecord;
use crate::paths::identity::FileIdentity;
use anyhow::Result;

pub(crate) fn validate_publication(
    namespace: &WorkerNamespace,
    paths: &Paths,
    record: &RuntimeRecord,
    authority: &StartupPredecessorAuthority,
) -> Result<FileIdentity> {
    let path = paths.worker_startup_checkpoint(record.terminal_id);
    let (checkpoint, checkpoint_identity): (StartupCheckpointEvidence, _) =
        namespace.read_json_with_identity(&path)?;
    super::validation::validate(&checkpoint)?;
    anyhow::ensure!(
        checkpoint.status == StartupStatus::Active
            && checkpoint_identity == authority.checkpoint_identity
            && super::fence::from_evidence(&checkpoint) == authority.successor
            && super::fence::matches_runtime(&checkpoint, record)?,
        super::constants::PREDECESSOR_PUBLICATION_ERROR
    );
    let (predecessor, identity): (RuntimeRecord, _) =
        namespace.read_json_with_identity(&paths.runtime_record(record.terminal_id))?;
    anyhow::ensure!(
        super::predecessor::runtime_digest(&predecessor)? == authority.runtime_sha256,
        super::constants::PREDECESSOR_PUBLICATION_ERROR
    );
    validate_custody(
        namespace,
        paths,
        predecessor.terminal_id,
        &authority.custody,
    )?;
    Ok(identity)
}

pub(crate) fn validate_custody(
    namespace: &WorkerNamespace,
    paths: &Paths,
    terminal_id: uuid::Uuid,
    authority: &super::StartupPredecessorCustodyAuthority,
) -> Result<()> {
    let path = paths.pty_custody(terminal_id);
    let current: Option<(crate::pty_custody::CustodyDurableRecord, _)> =
        if namespace.file_is_missing(&path)? {
            None
        } else {
            Some(namespace.read_json_with_identity(&path)?)
        };
    match (authority, current) {
        (super::StartupPredecessorCustodyAuthority::Absent, None) => Ok(()),
        (
            super::StartupPredecessorCustodyAuthority::Present { identity, sha256 },
            Some(current),
        ) => validate_present(current, *identity, *sha256),
        _ => anyhow::bail!(super::constants::PREDECESSOR_PUBLICATION_ERROR),
    }
}

pub(crate) fn validate_present(
    current: (crate::pty_custody::CustodyDurableRecord, FileIdentity),
    identity: FileIdentity,
    sha256: crate::sha256_digest::Sha256Digest,
) -> Result<()> {
    let (custody, current_identity) = current;
    anyhow::ensure!(
        current_identity == identity
            && crate::pty_custody::durable_record_digest(&custody)? == sha256,
        super::constants::PREDECESSOR_PUBLICATION_ERROR
    );
    Ok(())
}

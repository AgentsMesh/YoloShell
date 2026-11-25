use crate::{JournalError, constants::*, fresh_validation, storage};
use anyhow::{Context, Result, ensure};
use serde::de::DeserializeOwned;
use std::path::PathBuf;
use yolo_agent_control_journal::JournalScope;
use yolo_paths::{
    Paths,
    constants::{MAX_DURABLE_JSON_BYTES, WORKER_STARTUP_CHECKPOINT_MAX_BYTES},
    directory::PrivateDirectory,
    identity::FileIdentity,
    namespace_lease::NamespaceLease,
};
use yolo_terminal_model::{RuntimeRecord, TerminalManifest, WorkerLaunchSpec};
use yolo_startup_checkpoint_contract::StartupCheckpointEvidence;

/// Retained initializer evidence, never inferred from an absent journal or anchor.
/// This token does not prove process liveness or authorize a provider effect.
pub struct FreshInitialization {
    pub(crate) namespace: NamespaceLease,
    pub(crate) paths: Paths,
    pub(crate) runtime: RuntimeRecord,
    observed: Vec<ObservedFile>,
}
struct ObservedFile {
    path: PathBuf,
    identity: FileIdentity,
    limit: u64,
}

impl FreshInitialization {
    pub fn capture(
        session: PrivateDirectory,
        paths: &Paths,
        current: &RuntimeRecord,
    ) -> Result<Self, JournalError> {
        JournalScope::new(current.terminal_id, current.runtime_epoch)?;
        let namespace = storage::retain(session, paths, current.terminal_id)?;
        let id = current.terminal_id;
        let (durable, runtime): (RuntimeRecord, _) =
            read(&namespace, paths.runtime_record(id), MAX_DURABLE_JSON_BYTES)
                .map_err(|_| JournalError::Unavailable)?;
        let (launch, launch_file): (WorkerLaunchSpec, _) =
            read(&namespace, paths.worker_launch(id), MAX_DURABLE_JSON_BYTES)
                .map_err(|_| JournalError::Unavailable)?;
        let (checkpoint, checkpoint_file): (StartupCheckpointEvidence, _) = read(
            &namespace,
            paths.worker_startup_checkpoint(id),
            WORKER_STARTUP_CHECKPOINT_MAX_BYTES,
        )
        .map_err(|_| JournalError::Unavailable)?;
        let (manifest, manifest_file): (TerminalManifest, _) =
            read(&namespace, paths.manifest(id), MAX_DURABLE_JSON_BYTES)
                .map_err(|_| JournalError::Unavailable)?;
        fresh_validation::validate(current, &durable, &launch, &checkpoint, &manifest)
            .map_err(|_| JournalError::ScopeMismatch)?;
        let proof = Self {
            namespace,
            paths: paths.clone(),
            runtime: current.clone(),
            observed: vec![runtime, launch_file, checkpoint_file, manifest_file],
        };
        proof.verify().map_err(|_| JournalError::Unavailable)?;
        Ok(proof)
    }

    pub(crate) fn verify(&self) -> Result<()> {
        self.namespace.verify()?;
        for observed in &self.observed {
            self.namespace
                .file(&observed.path)?
                .verify_exact_identity(observed.identity, observed.limit)?;
        }
        self.namespace.verify()
    }
}

fn read<T: DeserializeOwned>(
    namespace: &NamespaceLease,
    path: PathBuf,
    limit: u64,
) -> Result<(T, ObservedFile)> {
    let expected = namespace
        .file(&path)?
        .inspect(limit)?
        .context(INVALID_INITIALIZATION)?;
    let (value, identity) = namespace.read_json_with_identity(&path)?;
    ensure!(identity == expected, CHANGED_INITIALIZATION);
    Ok((
        value,
        ObservedFile {
            path,
            identity,
            limit,
        },
    ))
}

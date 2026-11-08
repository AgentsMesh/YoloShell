use super::*;
use crate::paths::identity::FileIdentity;

pub(crate) const CODEX_RAW_AUTHORITY_REPLAY_CHANGED: &str =
    "Codex raw authority replay boundary changed";

impl Host {
    pub(crate) fn remove_raw_codex_authority(
        &self,
        guard: &yolo_paths::durable_mutation::SessionDurableMutationGuard<'_>,
        store: &CodexBindingAuthorityStore,
        expected: FileIdentity,
        transaction: &ResumeTransaction,
        marker: &ResumeBindingClearMarker,
    ) -> Result<()> {
        self.remove_raw_codex_authority_with_boundary(
            guard,
            store,
            expected,
            transaction,
            marker,
            || Ok(()),
        )
    }

    pub(crate) fn remove_raw_codex_authority_with_boundary<F>(
        &self,
        guard: &yolo_paths::durable_mutation::SessionDurableMutationGuard<'_>,
        store: &CodexBindingAuthorityStore,
        expected: FileIdentity,
        transaction: &ResumeTransaction,
        marker: &ResumeBindingClearMarker,
        mut boundary: F,
    ) -> Result<()>
    where
        F: FnMut() -> Result<()>,
    {
        guard.remove_exact_with_replay_boundary(
            store.path(),
            expected,
            RESUME_BINDING_EVIDENCE_MAX_BYTES,
            || {
                boundary()?;
                self.require_raw_codex_authority_replay(guard, transaction, marker)
            },
        )
    }

    pub(crate) fn require_raw_codex_authority_replay(
        &self,
        guard: &yolo_paths::durable_mutation::SessionDurableMutationGuard<'_>,
        transaction: &ResumeTransaction,
        marker: &ResumeBindingClearMarker,
    ) -> Result<()> {
        guard.require_no_pending_intent()?;
        transaction.validate_for(transaction.terminal_id)?;
        marker.validate_for(transaction)?;
        anyhow::ensure!(
            marker.phase == ResumeBindingClearPhase::Pending,
            CODEX_RAW_AUTHORITY_REPLAY_CHANGED
        );
        let transaction_path = self.paths.resume_transaction(transaction.terminal_id);
        let marker_path = self.paths.resume_binding_clear(transaction.terminal_id);
        let launch_path = self.paths.worker_launch(transaction.terminal_id);
        let (_, transaction_identity) =
            read_exact_raw_codex_replay_artifact(guard, &transaction_path, transaction)?;
        let (_, marker_identity) =
            read_exact_raw_codex_replay_artifact(guard, &marker_path, marker)?;
        let (launch, launch_identity) =
            read_exact_raw_codex_replay_artifact(guard, &launch_path, &transaction.launch)?;
        transaction.validate_launch(&launch)?;
        require_raw_codex_replay_artifact_unchanged(
            guard,
            &transaction_path,
            transaction,
            transaction_identity,
        )?;
        require_raw_codex_replay_artifact_unchanged(guard, &marker_path, marker, marker_identity)?;
        require_raw_codex_replay_artifact_unchanged(
            guard,
            &launch_path,
            &transaction.launch,
            launch_identity,
        )?;
        guard.require_no_pending_intent()
    }
}

pub(crate) fn read_exact_raw_codex_replay_artifact<T>(
    guard: &yolo_paths::durable_mutation::SessionDurableMutationGuard<'_>,
    path: &Path,
    expected: &T,
) -> Result<(T, FileIdentity)>
where
    T: serde::de::DeserializeOwned + PartialEq,
{
    let persisted = guard
        .read_optional_with_identity::<T>(path)?
        .context(CODEX_RAW_AUTHORITY_REPLAY_CHANGED)?;
    anyhow::ensure!(&persisted.0 == expected, CODEX_RAW_AUTHORITY_REPLAY_CHANGED);
    Ok(persisted)
}

pub(crate) fn require_raw_codex_replay_artifact_unchanged<T>(
    guard: &yolo_paths::durable_mutation::SessionDurableMutationGuard<'_>,
    path: &Path,
    expected: &T,
    expected_identity: FileIdentity,
) -> Result<()>
where
    T: serde::de::DeserializeOwned + PartialEq,
{
    let (_, current_identity) = read_exact_raw_codex_replay_artifact(guard, path, expected)?;
    anyhow::ensure!(
        current_identity.unchanged(expected_identity),
        CODEX_RAW_AUTHORITY_REPLAY_CHANGED
    );
    Ok(())
}

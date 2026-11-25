use super::{Host, RecoveryState, SESSION_MANIFEST_SCHEMA_VERSION, TerminalManifest};
use crate::model::TerminalManifestDigest;
use crate::paths::durable_mutation::SessionDurableMutationGuard;
use crate::paths::identity::FileIdentity;
use anyhow::{Context, Result, ensure};
use uuid::Uuid;

const RECOVERY_MANIFEST_INVALID: &str = "recovery publication requires an active session manifest";
const RECOVERY_MANIFEST_CHANGED: &str = "recovery publication original manifest changed";
const RECOVERY_MANIFEST_FIELDS: &str = "recovery publication changed non-recovery manifest fields";
const RECOVERY_MANIFEST_MISSING: &str = "recovery publication manifest is missing";
const RECOVERY_CATALOG_FAILED: &str = "catalog publication failed after recovery manifest commit";
const RECOVERY_DOWNGRADE_UNAVAILABLE: &str = "prepare committed recovery uncertainty";
const RECOVERY_CATALOG_TARGET_CHANGED: &str = "committed recovery belongs to another session";

pub(crate) struct RecoveryCatalogFailure {
    committed: TerminalManifest,
    identity: FileIdentity,
}

impl RecoveryCatalogFailure {
    pub(crate) fn new(committed: TerminalManifest, identity: FileIdentity) -> Self {
        Self {
            committed,
            identity,
        }
    }

    pub(crate) fn downgrade(&self, host: &Host, terminal_id: Uuid) -> Result<TerminalManifest> {
        ensure!(
            self.committed.terminal_id == terminal_id,
            RECOVERY_CATALOG_TARGET_CHANGED
        );
        let candidate = host
            .prepare_recovery_state(&self.committed, RecoveryState::Uncertain)
            .context(RECOVERY_DOWNGRADE_UNAVAILABLE)?;
        PreparedRecoveryManifest::capture_observed(
            host,
            &self.committed,
            self.identity,
            &candidate,
        )?
        .commit(host)?;
        Ok(candidate)
    }
}

impl std::fmt::Display for RecoveryCatalogFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(RECOVERY_CATALOG_FAILED)
    }
}

impl std::fmt::Debug for RecoveryCatalogFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, formatter)
    }
}

pub(crate) struct PreparedRecoveryManifest {
    terminal_id: Uuid,
    original_digest: TerminalManifestDigest,
    original_identity: FileIdentity,
    recovery_state: RecoveryState,
    updated_at_ms: i64,
}

impl PreparedRecoveryManifest {
    pub(crate) fn capture_observed(
        host: &Host,
        original: &TerminalManifest,
        original_identity: FileIdentity,
        candidate: &TerminalManifest,
    ) -> Result<Self> {
        let prepared = Self::capture(host, original, candidate)?;
        ensure!(
            prepared.original_identity == original_identity,
            RECOVERY_MANIFEST_CHANGED
        );
        Ok(prepared)
    }

    pub(crate) fn capture(
        host: &Host,
        original: &TerminalManifest,
        candidate: &TerminalManifest,
    ) -> Result<Self> {
        ensure!(
            original.schema_version == SESSION_MANIFEST_SCHEMA_VERSION
                && !original.terminal_id.is_nil()
                && !original.archived,
            RECOVERY_MANIFEST_INVALID
        );
        let original_digest = TerminalManifestDigest::capture(original)?;
        let mut unchanged = candidate.clone();
        unchanged.recovery_state = original.recovery_state;
        unchanged.updated_at_ms = original.updated_at_ms;
        ensure!(
            TerminalManifestDigest::capture(&unchanged)? == original_digest,
            RECOVERY_MANIFEST_FIELDS
        );
        let path = host.paths.manifest(original.terminal_id);
        let (current, original_identity): (TerminalManifest, FileIdentity) =
            host.namespace.read_json_with_identity(&path)?;
        ensure!(
            TerminalManifestDigest::capture(&current)? == original_digest,
            RECOVERY_MANIFEST_CHANGED
        );
        Ok(Self {
            terminal_id: original.terminal_id,
            original_digest,
            original_identity,
            recovery_state: candidate.recovery_state,
            updated_at_ms: candidate.updated_at_ms,
        })
    }

    pub(crate) fn commit(self, host: &Host) -> Result<FileIdentity> {
        // Callers may already hold lifecycle. This state-only CAS takes the exclusive
        // session directory lock and completes before any catalog work or RPC.
        let guard = SessionDurableMutationGuard::acquire_after_lifecycle_lock(
            &host.paths,
            &host.namespace,
            self.terminal_id,
        )?;
        guard.require_no_pending_intent()?;
        let path = host.paths.manifest(self.terminal_id);
        let (mut current, identity): (TerminalManifest, FileIdentity) = guard
            .read_optional_with_identity(&path)?
            .context(RECOVERY_MANIFEST_MISSING)?;
        ensure!(
            identity == self.original_identity
                && TerminalManifestDigest::capture(&current)? == self.original_digest,
            RECOVERY_MANIFEST_CHANGED
        );
        current.recovery_state = self.recovery_state;
        current.updated_at_ms = self.updated_at_ms;
        if TerminalManifestDigest::capture(&current)? == self.original_digest {
            guard.require_no_pending_intent()?;
            return Ok(identity);
        }
        let installed =
            guard.write_json_expected_with_commit_fence(&path, Some(identity), &current)?;
        guard.require_no_pending_intent()?;
        Ok(installed)
    }
}

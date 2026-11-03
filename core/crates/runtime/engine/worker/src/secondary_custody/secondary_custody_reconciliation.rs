use super::{
    ReadyWorkerCustodyIdentity, WorkerSecondaryCustodyAuthority,
    WorkerSecondaryCustodyAuthorityState,
};
use crate::model::{RuntimeRecord, TerminalManifest, WorkerLaunchSpec};
use crate::paths::identity::FileIdentity;
use crate::pty_custody::CustodyDurableRecord;
use crate::worker::{LeaseReason, SystemMutationPermit, WorkerState, lock_recover};
use anyhow::{Context, Result, ensure};
use std::sync::atomic::Ordering;
use yolo_paths::durable_mutation::SessionDurableMutationGuard;

const COMMITTED_CUSTODY_DETAIL: &str =
    "committed PTY custody successor reconciliation is not exact";
const COMMITTED_RUNTIME_DETAIL: &str = "committed PTY custody runtime reconciliation is not exact";
const COMMITTED_PREDECESSOR_DETAIL: &str =
    "committed PTY custody reconciliation predecessor changed";
const COMMITTED_MANIFEST_DETAIL: &str = "committed PTY custody reconciliation session changed";
const COMMITTED_TERMINATION_DETAIL: &str =
    "committed PTY custody reconciliation cannot replace a terminating controller";

pub(crate) struct CommittedCustodyAdoption {
    runtime: RuntimeRecord,
    durable: CustodyDurableRecord,
    identity: FileIdentity,
}

pub(crate) struct PendingCustodyReconciliation {
    predecessor: ReadyWorkerCustodyIdentity,
    committed: CommittedCustodyAdoption,
}

impl CommittedCustodyAdoption {
    pub(crate) fn new(
        runtime: RuntimeRecord,
        durable: CustodyDurableRecord,
        identity: FileIdentity,
    ) -> Self {
        Self {
            runtime,
            durable,
            identity,
        }
    }
}

impl WorkerSecondaryCustodyAuthority {
    pub(crate) fn retain_committed_adoption(
        &self,
        predecessor: Option<ReadyWorkerCustodyIdentity>,
        committed: CommittedCustodyAdoption,
    ) {
        if let Some(predecessor) = predecessor {
            *lock_recover(&self.pending) = Some(PendingCustodyReconciliation {
                predecessor,
                committed,
            });
        }
    }
}

impl WorkerState {
    pub(crate) fn reconcile_committed_custody(&self, permit: &SystemMutationPermit) -> Result<()> {
        let authority = &self.secondary_custody_authority;
        let mut pending = lock_recover(&authority.pending);
        let Some(reconciliation) = pending.as_ref() else {
            return Ok(());
        };
        let committed = &reconciliation.committed;
        let candidate = committed
            .durable
            .armed_lease()
            .context(COMMITTED_CUSTODY_DETAIL)?;
        let mut authority_state = lock_recover(&authority.state);
        ensure!(
            matches!(*authority_state, WorkerSecondaryCustodyAuthorityState::Disconnected { identity, .. }
                if identity == reconciliation.predecessor),
            COMMITTED_PREDECESSOR_DETAIL
        );
        let mut current = lock_recover(&self.record);
        ensure!(
            committed.durable.is_valid()
                && committed.durable.runtime.matches_record(&committed.runtime)
                && candidate.controller_generation > committed.runtime.controller_generation
                && candidate.controller_generation >= reconciliation.predecessor.generation(),
            COMMITTED_CUSTODY_DETAIL
        );
        ensure!(
            current.validate_update_from(&committed.runtime).is_ok()
                && current.controller_generation <= candidate.controller_generation
                && self.controller_generation.load(Ordering::Acquire)
                    <= candidate.controller_generation
                && self.controller_persisted_generation.load(Ordering::Acquire)
                    <= candidate.controller_generation,
            COMMITTED_RUNTIME_DETAIL
        );
        let guard = SessionDurableMutationGuard::acquire_participant(
            &self.paths,
            self.namespace.lease(),
            current.terminal_id,
        )?;
        guard.require_no_pending_intent()?;
        let terminal_id = current.terminal_id;
        let installed = guard
            .read_optional_with_identity::<CustodyDurableRecord>(
                &self.paths.pty_custody(terminal_id),
            )?
            .context(COMMITTED_CUSTODY_DETAIL)?;
        ensure!(
            installed == (committed.durable.clone(), committed.identity),
            COMMITTED_CUSTODY_DETAIL
        );
        let manifest: TerminalManifest = guard
            .read_optional_with_identity(&self.paths.manifest(terminal_id))?
            .map(|(value, _)| value)
            .context(COMMITTED_MANIFEST_DETAIL)?;
        ensure!(
            manifest.schema_version == crate::constants::SESSION_MANIFEST_SCHEMA_VERSION
                && manifest.terminal_id == terminal_id
                && !manifest.archived
                && guard
                    .read_optional_with_identity::<WorkerLaunchSpec>(
                        &self.paths.worker_launch(terminal_id),
                    )?
                    .is_none(),
            COMMITTED_MANIFEST_DETAIL
        );
        let published: RuntimeRecord = guard
            .read_optional_with_identity(&self.paths.runtime_record(terminal_id))?
            .map(|(value, _)| value)
            .context(COMMITTED_RUNTIME_DETAIL)?;
        ensure!(
            published.validate_update_from(&current).is_ok()
                && published.controller_generation
                    >= self.controller_generation.load(Ordering::Acquire)
                && published.controller_generation
                    >= self.controller_persisted_generation.load(Ordering::Acquire)
                && published.controller_generation <= candidate.controller_generation,
            COMMITTED_RUNTIME_DETAIL
        );
        guard.require_no_pending_intent()?;
        ensure!(!self.has_termination_intent(), COMMITTED_TERMINATION_DETAIL);
        self.namespace.verify()?;
        let successor = ReadyWorkerCustodyIdentity::from_durable(candidate);
        let generation = published.controller_generation;
        // An Armed generation is reserved, not activated. Never rewrite it or grant control.
        *current = published;
        self.controller_generation
            .store(generation, Ordering::Release);
        self.controller_persisted_generation
            .store(generation, Ordering::Release);
        *authority_state = WorkerSecondaryCustodyAuthorityState::Disconnected {
            identity: successor,
            profile: candidate.profile,
        };
        drop(current);
        drop(authority_state);
        self.revoke_writer_lease_admitted(permit, LeaseReason::ControllerReplaced);
        *pending = None;
        Ok(())
    }
}

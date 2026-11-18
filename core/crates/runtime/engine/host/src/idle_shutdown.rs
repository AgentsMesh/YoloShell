use super::request_drain::HostDrainPreparation;
use super::*;
use crate::protocol::ErrorBody;
use tokio::sync::{OwnedMutexGuard, OwnedSemaphorePermit, Semaphore};

const ACTIVE_ATTACHMENTS: &str = "host has active terminal attachments";
const ACTIVE_CLIENTS: &str = "host accepted a client during automatic idle shutdown";
const ACTIVE_CREATION: &str = "host has an active session creation transaction";
const ACTIVE_RESUME: &str = "host has an active session recovery transaction";
const PENDING_RECOVERY_WRITE: &str = "host has an uncommitted recovery state write";
const ACTIVE_SESSION: &str = "host has a live or indeterminate session";
const CATALOG_UNCERTAIN: &str = "host catalog contains an indeterminate row";

pub(super) struct HostIdleShutdown {
    drain: HostDrainPreparation,
    _reconcile: OwnedMutexGuard<()>,
}

pub(super) struct HostQuiescentShutdown {
    shutdown: HostIdleShutdown,
    _clients: OwnedSemaphorePermit,
}

impl Host {
    pub(super) async fn prepare_idle_shutdown(
        &self,
    ) -> std::result::Result<HostIdleShutdown, ErrorBody> {
        let drain = self.request_drain.prepare().await?;
        let reconcile = std::sync::Arc::clone(&self.reconcile_gate)
            .lock_owned()
            .await;
        self.verify_idle_shutdown().await?;
        Ok(HostIdleShutdown {
            drain,
            _reconcile: reconcile,
        })
    }

    pub(super) async fn prepare_quiescent_shutdown(
        &self,
        clients: std::sync::Arc<Semaphore>,
    ) -> std::result::Result<HostQuiescentShutdown, ErrorBody> {
        let drain = self.request_drain.prepare().await?;
        let permits = clients
            .try_acquire_many_owned(MAX_HOST_CLIENTS as u32)
            .map_err(|_| ErrorBody::conflict(ACTIVE_CLIENTS))?;
        let reconcile = std::sync::Arc::clone(&self.reconcile_gate)
            .lock_owned()
            .await;
        self.verify_idle_shutdown().await?;
        Ok(HostQuiescentShutdown {
            shutdown: HostIdleShutdown {
                drain,
                _reconcile: reconcile,
            },
            _clients: permits,
        })
    }

    async fn verify_idle_shutdown(&self) -> std::result::Result<(), ErrorBody> {
        if self.attachment_clients.available_permits() != MAX_HOST_ATTACHMENTS {
            return Err(ErrorBody::conflict(ACTIVE_ATTACHMENTS));
        }
        if !lock_recover(&self.creation_capabilities)
            .bindings
            .is_empty()
        {
            return Err(ErrorBody::conflict(ACTIVE_CREATION));
        }
        if !lock_recover(&self.parked_resume_retries).is_empty() {
            return Err(ErrorBody::conflict(ACTIVE_RESUME));
        }
        if !lock_recover(&self.pending_recovery_writes).is_empty() {
            return Err(ErrorBody::conflict(PENDING_RECOVERY_WRITE));
        }
        let listing = self.catalog_list_tolerant(true).await.map_err(internal)?;
        if !listing.diagnostics.is_empty() {
            return Err(ErrorBody::conflict(CATALOG_UNCERTAIN));
        }
        for manifest in listing.manifests {
            let archived = manifest.archived;
            let recovery_state = manifest.recovery_state;
            if self
                .load_resume_transaction(manifest.terminal_id)
                .map_err(internal)?
                .is_some()
            {
                return Err(ErrorBody::conflict(ACTIVE_RESUME));
            }
            if archived {
                // An archived terminal is settled once exact custody proof
                // shows no live runtime. Its retained recovery label cannot be
                // resolved by resume or reconcile and must not veto shutdown.
                if self
                    .require_archived_runtime_settled(manifest.terminal_id)
                    .is_err()
                {
                    return Err(ErrorBody::conflict(ACTIVE_SESSION));
                }
                continue;
            }
            let runtime = self.view_for_manifest(manifest).await.runtime;
            if matches!(
                recovery_state,
                RecoveryState::Restoring | RecoveryState::Uncertain | RecoveryState::NeedsRecreate
            ) {
                return Err(ErrorBody::conflict(ACTIVE_SESSION));
            }
            if runtime.map(|value| value.status) != Some(TerminalRuntimeStatus::Exited) {
                return Err(ErrorBody::conflict(ACTIVE_SESSION));
            }
        }
        Ok(())
    }
}

impl HostIdleShutdown {
    pub(super) fn commit(self) {
        self.drain.commit();
    }
}

impl HostQuiescentShutdown {
    pub(super) fn commit(self) {
        self.shutdown.commit();
    }
}

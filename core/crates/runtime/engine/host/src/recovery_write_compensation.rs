use super::*;

pub(super) const PENDING_RECOVERY_WRITE_RETRY_FAILED_DETAIL: &str = "durable recovery write is still pending";

impl Host {
    pub(super) fn record_pending_recovery_write(&self, manifest: &TerminalManifest, to: RecoveryState) {
        let Ok(expected) = TerminalManifestDigest::capture(manifest) else {
            return;
        };
        lock_recover(&self.pending_recovery_writes).insert(
            manifest.terminal_id,
            PendingRecoveryWrite { to, expected },
        );
    }

    pub(super) fn clear_pending_recovery_write(&self, terminal_id: Uuid) {
        lock_recover(&self.pending_recovery_writes).remove(&terminal_id);
    }

    pub(super) fn clear_pending_recovery_write_if(&self, terminal_id: Uuid, expected: &PendingRecoveryWrite) {
        let mut pending = lock_recover(&self.pending_recovery_writes);
        if pending.get(&terminal_id) == Some(expected) {
            pending.remove(&terminal_id);
        }
    }

    pub(super) fn clear_pending_recovery_write_for_digest(
        &self,
        terminal_id: Uuid,
        expected: &TerminalManifestDigest,
    ) {
        let mut pending = lock_recover(&self.pending_recovery_writes);
        if pending
            .get(&terminal_id)
            .is_some_and(|write| &write.expected == expected)
        {
            pending.remove(&terminal_id);
        }
    }

    pub(super) fn pending_recovery_write(&self, terminal_id: Uuid) -> Option<PendingRecoveryWrite> {
        lock_recover(&self.pending_recovery_writes)
            .get(&terminal_id)
            .cloned()
    }

    /// Retries recovery-state writes that outlived their original decision.
    /// Returns the number of sessions whose storage still rejects the write.
    pub(super) async fn retry_pending_recovery_writes(&self) -> usize {
        let pending = lock_recover(&self.pending_recovery_writes)
            .iter()
            .map(|(terminal_id, write)| (*terminal_id, write.clone()))
            .collect::<Vec<_>>();
        let mut failures = usize::default();
        for (terminal_id, write) in pending {
            if !self.retry_pending_recovery_write(terminal_id, write).await {
                failures += 1;
            }
        }
        failures
    }

    pub(super) async fn retry_pending_recovery_write(
        &self,
        terminal_id: Uuid,
        pending: PendingRecoveryWrite,
    ) -> bool {
        let _session_guard = self.session_locks.lock(terminal_id).await;
        // The map may have recorded a newer decision while this retry waited.
        if self.pending_recovery_write(terminal_id).as_ref() != Some(&pending) {
            return true;
        }
        let path = self.paths.manifest(terminal_id);
        let mut manifest = match self.load_durable_manifest(terminal_id) {
            Ok(manifest) => manifest,
            Err(error) => {
                if self.namespace.file_is_missing(&path).unwrap_or(false) {
                    self.clear_pending_recovery_write_if(terminal_id, &pending);
                    return true;
                }
                tracing::warn!(
                    %terminal_id,
                    %error,
                    PENDING_RECOVERY_WRITE_RETRY_FAILED_DETAIL
                );
                return false;
            }
        };
        // A newer durable generation or an archive supersedes the decision.
        let current = match TerminalManifestDigest::capture(&manifest) {
            Ok(current) => current,
            Err(error) => {
                tracing::warn!(
                    %terminal_id,
                    %error,
                    PENDING_RECOVERY_WRITE_RETRY_FAILED_DETAIL
                );
                return false;
            }
        };
        if manifest.archived || current != pending.expected {
            self.clear_pending_recovery_write_if(terminal_id, &pending);
            return true;
        }
        self.persist_recovery_state(&mut manifest, pending.to).await
    }
}

use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AutomaticRestore {
    Eligible,
    AwaitingRuntimeSettlement,
    BlockedByRuntime,
    Ineligible,
}

impl Host {
    pub(super) fn recovery_finished(
        &self,
        manifest: &TerminalManifest,
        record: &RuntimeRecord,
    ) -> bool {
        if record.terminal_id != manifest.terminal_id
            || !runtime_loss::identities_proven_gone(record, &self.boot_id)
        {
            return false;
        }
        let final_state = matches!(
            manifest.recovery_state,
            RecoveryState::Uncertain | RecoveryState::NeedsRecreate
        ) || self.termination_tombstone_state(record)
            == TerminationTombstoneState::Matching
            || matches!(self.exit_record_state(record), ExitRecordState::Matching(reason) if reason != ExitReason::WorkerCrash);
        final_state
            && self
                .load_resume_transaction(manifest.terminal_id)
                .is_ok_and(|transaction| transaction.is_none())
    }

    pub(super) fn automatic_restore_decision_for_runtime(
        &self,
        manifest: &TerminalManifest,
        expected: &RuntimeRecord,
    ) -> AutomaticRestore {
        let Ok(runtime) = self
            .read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(manifest.terminal_id))
        else {
            return AutomaticRestore::Ineligible;
        };
        if runtime != *expected {
            return AutomaticRestore::Ineligible;
        }
        self.classify_automatic_restore(manifest, &runtime)
    }

    pub(super) fn classify_automatic_restore(
        &self,
        manifest: &TerminalManifest,
        runtime: &RuntimeRecord,
    ) -> AutomaticRestore {
        if matches!(
            manifest.recovery_state,
            RecoveryState::NeedsRecreate | RecoveryState::Uncertain
        ) {
            return AutomaticRestore::Ineligible;
        }
        if runtime.schema_version != RUNTIME_RECORD_SCHEMA_VERSION
            || runtime.terminal_id != manifest.terminal_id
        {
            return AutomaticRestore::Ineligible;
        }
        if !runtime.bootstrap_delivery.is_ready()
            || !self.exit_allows_restore(runtime)
            || self.termination_tombstone_state(runtime) != TerminationTombstoneState::Missing
        {
            return AutomaticRestore::Ineligible;
        }
        match process::boot_identity_relation(&runtime.boot_id, &self.boot_id) {
            process::BootIdentityRelation::Different => {
                return if self
                    .validate_previous_resume_runtime_core(manifest, runtime)
                    .is_ok()
                {
                    AutomaticRestore::Eligible
                } else {
                    AutomaticRestore::Ineligible
                };
            }
            process::BootIdentityRelation::Unknown => {
                return AutomaticRestore::BlockedByRuntime;
            }
            process::BootIdentityRelation::Same => {}
        }
        if runtime.worker_start_time == 0 || runtime.child_start_time == 0 {
            return AutomaticRestore::BlockedByRuntime;
        }
        #[cfg(target_os = "linux")]
        {
            if process_identity_is_gone(runtime.worker_pid, runtime.worker_start_time)
                && self.linux_fallback_capabilities(runtime).is_some()
            {
                return AutomaticRestore::Eligible;
            }
            return AutomaticRestore::BlockedByRuntime;
        }
        #[cfg(not(target_os = "linux"))]
        {
            let custody = self.secondary_custody.recovery_admission(
                self,
                &crate::pty_custody::CustodyRuntimeIdentity::from_record(runtime),
            );
            match custody {
                secondary_custody::SecondaryCustodyRecoveryAdmission::AwaitingSettlement => {
                    AutomaticRestore::AwaitingRuntimeSettlement
                }
                secondary_custody::SecondaryCustodyRecoveryAdmission::Retired => {
                    let worker_gone =
                        process_identity_is_gone(runtime.worker_pid, runtime.worker_start_time);
                    let child_gone =
                        process_identity_is_gone(runtime.child_pid, runtime.child_start_time);
                    if worker_gone && child_gone {
                        AutomaticRestore::Eligible
                    } else {
                        AutomaticRestore::AwaitingRuntimeSettlement
                    }
                }
                secondary_custody::SecondaryCustodyRecoveryAdmission::Unavailable => {
                    AutomaticRestore::BlockedByRuntime
                }
            }
        }
    }

    #[cfg(test)]
    pub(super) fn runtime_is_resolved_for_spawn(&self, manifest: &TerminalManifest) -> bool {
        self.read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(manifest.terminal_id))
            .and_then(|runtime| self.validate_previous_resume_runtime(manifest, &runtime))
            .is_ok()
    }
}

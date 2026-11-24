use super::*;

#[cfg(target_os = "linux")]
pub(super) const UNKNOWN_BOOT_TERMINATION_DETAIL: &str =
    "runtime boot identity cannot authorize destructive recovery";

impl Host {
    #[cfg(test)]
    pub(super) fn persist_dead_runtime_termination(
        &self,
        terminal_id: Uuid,
        force: bool,
    ) -> std::result::Result<bool, ErrorBody> {
        let Ok(record) =
            self.read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(terminal_id))
        else {
            return Ok(false);
        };
        if !runtime_loss::runtime_record_identity_is_verifiable(&record, terminal_id) {
            return Ok(false);
        }
        let previous_boot = process::boot_identity_relation(&record.boot_id, &self.boot_id)
            == process::BootIdentityRelation::Different;
        let both_gone = process_identity_is_gone(record.worker_pid, record.worker_start_time)
            && process_identity_is_gone(record.child_pid, record.child_start_time);
        if !previous_boot && !both_gone {
            return Ok(false);
        }
        self.persist_runtime_termination(&record, force)?;
        Ok(true)
    }

    pub(super) async fn settle_unreachable_runtime_termination(
        &self,
        terminal_id: Uuid,
        force: bool,
        runtime_fence: Option<&LifecycleMutationFence>,
    ) -> std::result::Result<bool, ErrorBody> {
        let Ok(record) =
            self.read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(terminal_id))
        else {
            return Ok(false);
        };
        if !runtime_loss::runtime_record_identity_is_verifiable(&record, terminal_id) {
            return Ok(false);
        }
        if let Some(fence) = runtime_fence {
            fence.runtime.validate_observed(Some(&record))?;
        }
        match process::boot_identity_relation(&record.boot_id, &self.boot_id) {
            process::BootIdentityRelation::Different => {
                self.persist_runtime_termination(&record, force)?;
                #[cfg(target_os = "linux")]
                self.retire_rebooted_containment(&record)?;
                Ok(true)
            }
            process::BootIdentityRelation::Unknown => {
                #[cfg(target_os = "linux")]
                return Err(destructive_unavailable(UNKNOWN_BOOT_TERMINATION_DETAIL));
                #[cfg(not(target_os = "linux"))]
                Ok(false)
            }
            process::BootIdentityRelation::Same => {
                #[cfg(target_os = "linux")]
                {
                    if !process_identity_is_gone(record.worker_pid, record.worker_start_time) {
                        return Ok(false);
                    }
                    self.terminate_linux_containment(&record, force).await?;
                    Ok(true)
                }
                #[cfg(not(target_os = "linux"))]
                {
                    self.settle_disconnected_termination(
                        &record,
                        force,
                        PROCESS_EXIT_TIMEOUT,
                    )
                    .await
                }
            }
        }
    }

    pub(super) fn persist_runtime_termination(
        &self,
        record: &RuntimeRecord,
        force: bool,
    ) -> std::result::Result<(), ErrorBody> {
        let tombstone = crate::model::TerminationTombstone::user_request(
            record.terminal_id,
            record.runtime_epoch,
            force,
        );
        self.write_namespace_json(
            &self.paths.terminate_tombstone(record.terminal_id),
            &tombstone,
        )
        .map_err(internal)
    }
}

#[cfg(target_os = "linux")]
pub(super) fn destructive_unavailable(message: impl Into<String>) -> ErrorBody {
    ErrorBody::new(
        crate::protocol::error_code::DESTRUCTIVE_SESSION_UNAVAILABLE,
        message,
        true,
    )
}

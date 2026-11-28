use super::*;

pub(super) const LINUX_CONTAINMENT_READ_DETAIL: &str = "read durable Linux containment custody";
pub(super) const LINUX_CONTAINMENT_RECOVERY_DETAIL: &str = "recover dead Worker Linux containment";

impl Host {
    pub(super) async fn settle_linux_recreate_runtime(
        &self,
        runtime: &RuntimeRecord,
    ) -> std::result::Result<(), ErrorBody> {
        match process::boot_identity_relation(&runtime.boot_id, &self.boot_id) {
            process::BootIdentityRelation::Different => self.retire_rebooted_containment(runtime),
            process::BootIdentityRelation::Unknown => {
                Err(destructive_unavailable(RESUME_PREVIOUS_RUNTIME_UNRESOLVED))
            }
            process::BootIdentityRelation::Same => {
                if !process_identity_is_gone(runtime.worker_pid, runtime.worker_start_time) {
                    return Err(destructive_unavailable(RESUME_PREVIOUS_RUNTIME_UNRESOLVED));
                }
                self.recover_linux_containment(runtime).await.map(|_| ())
            }
        }
    }

    pub(super) async fn terminate_linux_containment(
        &self,
        runtime: &RuntimeRecord,
        force: bool,
    ) -> std::result::Result<(), ErrorBody> {
        let (path, record, leaf_removed) = self.recover_linux_containment(runtime).await?;
        self.persist_runtime_termination(runtime, force)?;
        self.retire_recovered_linux_record(&path, &record, leaf_removed)
    }

    pub(super) fn retire_rebooted_containment(
        &self,
        runtime: &RuntimeRecord,
    ) -> std::result::Result<(), ErrorBody> {
        let path = process::linux_containment::record_path(
            &self.paths.session_dir(runtime.terminal_id),
            runtime.runtime_epoch,
        );
        let Ok(record) =
            self.read_namespace_json::<process::linux_containment::LinuxContainmentRecord>(&path)
        else {
            return Ok(());
        };
        let identity_matches = record.terminal_id == runtime.terminal_id
            && record.runtime_epoch == runtime.runtime_epoch;
        let boot_changed = process::boot_identity_relation(&record.boot_id, &self.boot_id)
            == process::BootIdentityRelation::Different;
        if identity_matches && boot_changed {
            self.namespace
                .remove_json_if_matches(&path, &record)
                .map_err(internal)?;
        }
        Ok(())
    }

    pub(super) fn linux_fallback_capabilities(&self, runtime: &RuntimeRecord) -> Option<Vec<String>> {
        if process::boot_identity_relation(&runtime.boot_id, &self.boot_id)
            == process::BootIdentityRelation::Different
        {
            return Some(linux_termination_capability());
        }
        if !process_identity_is_gone(runtime.worker_pid, runtime.worker_start_time) {
            return None;
        }
        let path = process::linux_containment::record_path(
            &self.paths.session_dir(runtime.terminal_id),
            runtime.runtime_epoch,
        );
        let record = self
            .read_namespace_json::<process::linux_containment::LinuxContainmentRecord>(&path)
            .ok()?;
        let root = process::ProcessIdentity {
            pid: runtime.child_pid,
            start_time: runtime.child_start_time,
        };
        process::linux_containment::LinuxContainmentRecovery::open(
            record,
            runtime.terminal_id,
            runtime.runtime_epoch,
            root,
        )
        .ok()
        .map(|_| linux_termination_capability())
    }
}

pub(super) fn linux_containment_error(context: &str, error: impl std::fmt::Display) -> ErrorBody {
    destructive_unavailable(format!("{context}: {error}"))
}

pub(super) fn linux_termination_capability() -> Vec<String> {
    vec![crate::protocol::capability::SAFE_PROCESS_TREE_TERMINATION.to_owned()]
}

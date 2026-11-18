use super::*;

impl Host {
    pub(super) async fn recover_linux_containment(
        &self,
        runtime: &RuntimeRecord,
    ) -> std::result::Result<
        (
            PathBuf,
            process::linux_containment::LinuxContainmentRecord,
            bool,
        ),
        ErrorBody,
    > {
        let path = process::linux_containment::record_path(
            &self.paths.session_dir(runtime.terminal_id),
            runtime.runtime_epoch,
        );
        let record = self
            .read_namespace_json::<process::linux_containment::LinuxContainmentRecord>(&path)
            .map_err(|error| linux_containment_error(LINUX_CONTAINMENT_READ_DETAIL, error))?;
        let root = process::ProcessIdentity {
            pid: runtime.child_pid,
            start_time: runtime.child_start_time,
        };
        let mut recovery = process::linux_containment::LinuxContainmentRecovery::open(
            record.clone(),
            runtime.terminal_id,
            runtime.runtime_epoch,
            root,
        )
        .map_err(|error| linux_containment_error(LINUX_CONTAINMENT_RECOVERY_DETAIL, error))?;
        let deadline = tokio::time::Instant::now()
            + crate::session_timing::LINUX_CONTAINMENT_RECOVERY_TIMEOUT;
        loop {
            match recovery.poll().map_err(|error| {
                linux_containment_error(LINUX_CONTAINMENT_RECOVERY_DETAIL, error)
            })? {
                process::linux_containment::LinuxContainmentRecoveryPoll::Pending => {}
                process::linux_containment::LinuxContainmentRecoveryPoll::Settled {
                    leaf_removed,
                } => return Ok((path, record, leaf_removed)),
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(destructive_unavailable(LINUX_CONTAINMENT_RECOVERY_DETAIL));
            }
            tokio::time::sleep(PROCESS_EXIT_POLL_INTERVAL).await;
        }
    }

    pub(super) fn retire_recovered_linux_record(
        &self,
        path: &Path,
        record: &process::linux_containment::LinuxContainmentRecord,
        leaf_removed: bool,
    ) -> std::result::Result<(), ErrorBody> {
        if leaf_removed {
            self.namespace
                .remove_json_if_matches(path, record)
                .map_err(internal)?;
        }
        Ok(())
    }
}

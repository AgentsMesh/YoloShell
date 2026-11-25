pub(crate) const LIFECYCLE_MUTATION_BUSY: &str =
    "session lifecycle mutation conflicts with runtime publication";

#[derive(Debug)]
pub(crate) enum LifecycleRuntimeFence {
    Unconditional,
    Absent,
    Selected(RuntimeRecord),
}

#[derive(Debug)]
pub(crate) struct LifecycleMutationFence {
    pub(crate) runtime: LifecycleRuntimeFence,
    pub(crate) selection: TerminalLifecycleSelection,
}

impl LifecycleRuntimeFence {
    pub(crate) fn validate_observed(
        &self,
        current: Option<&RuntimeRecord>,
    ) -> std::result::Result<(), ErrorBody> {
        let matches = match (self, current) {
            (Self::Unconditional, _) | (Self::Absent, None) => true,
            (Self::Selected(expected), Some(current)) => {
                current.has_same_process_identity(expected)
            }
            (Self::Absent | Self::Selected(_), _) => false,
        };
        if matches {
            Ok(())
        } else {
            Err(ErrorBody::conflict(LIFECYCLE_RUNTIME_EPOCH_CHANGED))
        }
    }
}

impl Host {
    pub(crate) async fn acquire_resume_lifecycle_lock(
        &self,
        terminal_id: Uuid,
        selection: Option<&TerminalLifecycleSelection>,
    ) -> std::result::Result<crate::paths::private_file_lock::PrivateFileLock, ErrorBody> {
        let lifecycle_lock = self.wait_for_resume_lifecycle_lock(terminal_id).await?;
        let Some(selection) = selection else {
            self.require_no_pending_lifecycle_runtime(
                terminal_id,
                &LifecycleRuntimeFence::Unconditional,
            )?;
            return Ok(lifecycle_lock);
        };
        let fence = self.revalidate_lifecycle_fence(selection).await?;
        self.require_current_lifecycle_runtime(selection.terminal_id, &fence.runtime)?;
        self.require_no_pending_lifecycle_runtime(selection.terminal_id, &fence.runtime)?;
        self.validate_product_selection(selection).await?;
        Ok(lifecycle_lock)
    }

    pub(crate) async fn acquire_lifecycle_mutation_fence(
        &self,
        selection: &TerminalLifecycleSelection,
    ) -> std::result::Result<
        (
            crate::paths::private_file_lock::PrivateFileLock,
            LifecycleMutationFence,
        ),
        ErrorBody,
    > {
        let lifecycle_lock = self
            .wait_for_resume_lifecycle_lock(selection.terminal_id)
            .await?;
        let fence = self.revalidate_lifecycle_fence(selection).await?;
        self.require_current_lifecycle_runtime(selection.terminal_id, &fence.runtime)?;
        self.require_no_pending_lifecycle_runtime(selection.terminal_id, &fence.runtime)?;
        Ok((lifecycle_lock, fence))
    }

    pub(crate) fn capture_lifecycle_mutation_runtime(
        &self,
        selection: &TerminalLifecycleSelection,
    ) -> std::result::Result<LifecycleRuntimeFence, ErrorBody> {
        if let Some(runtime) = self.capture_runtime_epoch_fence(selection)? {
            return Ok(LifecycleRuntimeFence::Selected(runtime));
        }
        if selection.require_runtime_absent {
            return Ok(LifecycleRuntimeFence::Absent);
        }
        self.capture_current_lifecycle_runtime(selection.terminal_id)
    }

    pub(crate) fn capture_current_lifecycle_runtime(
        &self,
        terminal_id: Uuid,
    ) -> std::result::Result<LifecycleRuntimeFence, ErrorBody> {
        let path = self.paths.runtime_record(terminal_id);
        match self.read_namespace_json::<RuntimeRecord>(&path) {
            Ok(runtime) if lifecycle_runtime_is_valid(&runtime, terminal_id) => {
                Ok(LifecycleRuntimeFence::Selected(runtime))
            }
            Err(_) if lifecycle_path_is_absent(&path) => Ok(LifecycleRuntimeFence::Absent),
            Ok(_) | Err(_) => Err(ErrorBody::conflict(LIFECYCLE_RUNTIME_EPOCH_CHANGED)),
        }
    }

    pub(crate) fn require_current_lifecycle_runtime(
        &self,
        terminal_id: Uuid,
        fence: &LifecycleRuntimeFence,
    ) -> std::result::Result<(), ErrorBody> {
        if matches!(fence, LifecycleRuntimeFence::Unconditional) {
            return Ok(());
        }
        let path = self.paths.runtime_record(terminal_id);
        match self.read_namespace_json::<RuntimeRecord>(&path) {
            Ok(current) => fence.validate_observed(Some(&current)),
            Err(_) if !lifecycle_path_is_absent(&path) => {
                Err(ErrorBody::conflict(LIFECYCLE_RUNTIME_EPOCH_CHANGED))
            }
            Err(_) => fence.validate_observed(None),
        }
    }

    pub(crate) fn require_no_pending_lifecycle_runtime(
        &self,
        terminal_id: Uuid,
        fence: &LifecycleRuntimeFence,
    ) -> std::result::Result<(), ErrorBody> {
        let runtime = match fence {
            LifecycleRuntimeFence::Unconditional => {
                let path = self.paths.runtime_record(terminal_id);
                match self.read_namespace_json::<RuntimeRecord>(&path) {
                    Ok(runtime) => Some(runtime),
                    Err(_) if lifecycle_path_is_absent(&path) => None,
                    Err(_) => {
                        return Err(ErrorBody::conflict(LIFECYCLE_RUNTIME_EPOCH_CHANGED));
                    }
                }
            }
            LifecycleRuntimeFence::Absent => None,
            LifecycleRuntimeFence::Selected(runtime) => Some(runtime.clone()),
        };
        let checkpoint_allows = crate::worker::worker_startup_checkpoint_allows_lifecycle_mutation(
            &self.paths,
            terminal_id,
            runtime.as_ref(),
        )
        .map_err(|_| ErrorBody::conflict(LIFECYCLE_RUNTIME_EPOCH_CHANGED))?;
        if !checkpoint_allows
            || !lifecycle_path_is_absent(&self.paths.resume_transaction(terminal_id))
        {
            return Err(ErrorBody::conflict(LIFECYCLE_RUNTIME_EPOCH_CHANGED));
        }
        Ok(())
    }
}

pub(crate) fn lifecycle_runtime_is_valid(runtime: &RuntimeRecord, terminal_id: Uuid) -> bool {
    runtime.schema_version == RUNTIME_RECORD_SCHEMA_VERSION
        && runtime.terminal_id == terminal_id
        && !runtime.runtime_epoch.is_nil()
        && !runtime.worker_id.is_nil()
        && runtime.worker_pid != 0
        && runtime.worker_start_time != 0
        && runtime.child_pid != 0
        && runtime.child_start_time != 0
}

pub(crate) fn lifecycle_path_is_absent(path: &Path) -> bool {
    matches!(
        std::fs::symlink_metadata(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound
    )
}

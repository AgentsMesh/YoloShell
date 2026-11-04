pub(crate) const REQUEST_RUNTIME_MISMATCH_DETAIL: &str = "request targets a different worker runtime epoch";

impl WorkerState {
    pub(crate) fn terminal_id(&self) -> Uuid {
        lock_recover(&self.record).terminal_id
    }

    pub(crate) fn runtime_epoch(&self) -> Uuid {
        lock_recover(&self.record).runtime_epoch
    }

    pub(crate) fn set_agent_title(&self, title: Option<String>) {
        *lock_recover(&self.agent_title) = title;
    }

    pub(crate) fn mark_output_blocked(&self, cause: OutputBlockCause) -> bool {
        self.output_block.mark(cause)
    }

    pub(crate) fn clear_output_blocked(&self, cause: OutputBlockCause) {
        self.output_block.clear(cause);
    }

    pub(crate) fn output_blocked(&self) -> bool {
        self.output_block.blocked()
    }

    /// The display title prefers the active provider's conversation title and
    /// falls back to the PTY's OSC 0/2 title. This is presentation metadata
    /// only; it grants no agent, terminal or control authority.
    pub(crate) fn presentation(&self) -> crate::model::TerminalPresentation {
        let mut presentation = lock_recover(&self.terminal)
            .callbacks()
            .presentation()
            .clone();
        if let Some(title) = lock_recover(&self.agent_title).as_ref() {
            presentation.title = Some(title.clone());
        }
        presentation
    }

    pub(crate) fn status(&self) -> RuntimeView {
        let record = lock_recover(&self.record).clone();
        let exit = lock_recover(&self.exit).clone();
        RuntimeView {
            runtime_epoch: record.runtime_epoch,
            worker_id: record.worker_id,
            worker_pid: record.worker_pid,
            worker_start_time: record.worker_start_time,
            child_pid: record.child_pid,
            child_start_time: record.child_start_time,
            status: if exit.is_some() {
                TerminalRuntimeStatus::Exited
            } else if self.output_blocked() {
                TerminalRuntimeStatus::OutputBlocked
            } else if self.is_finalizing() {
                TerminalRuntimeStatus::Finalizing
            } else if !self.ready.load(Ordering::Acquire) {
                TerminalRuntimeStatus::Starting
            } else {
                TerminalRuntimeStatus::Running
            },
            last_output_sequence: lock_recover(&self.journal).last_sequence(),
            presentation: self.presentation(),
            exit,
            bootstrap_delivery: record.bootstrap_delivery,
            capabilities: Some(self.runtime_capabilities()),
            fault_reap: self
                .fault_reap_observer
                .get()
                .and_then(pty_fault_reaper::FaultReapStateObserver::status),
        }
    }

    pub(crate) fn validate_session(&self, terminal_id: Uuid) -> std::result::Result<(), ErrorBody> {
        if terminal_id == self.terminal_id() {
            Ok(())
        } else {
            Err(ErrorBody::invalid(
                "request targets a different worker session",
            ))
        }
    }

    pub(crate) fn validate_attachment_cursor(&self, after_seq: u64) -> std::result::Result<(), ErrorBody> {
        let last = lock_recover(&self.journal).last_sequence();
        if after_seq == FORCE_SNAPSHOT_AFTER_SEQUENCE || after_seq <= last {
            return Ok(());
        }
        Err(ErrorBody::invalid(ATTACHMENT_CURSOR_AHEAD_CONTEXT))
    }

    pub(crate) fn validate_runtime(
        &self,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
    ) -> std::result::Result<(), ErrorBody> {
        let record = lock_recover(&self.record);
        if record.terminal_id == terminal_id && record.runtime_epoch == runtime_epoch {
            return Ok(());
        }
        Err(ErrorBody::new(
            crate::protocol::error_code::IDENTITY_MISMATCH,
            REQUEST_RUNTIME_MISMATCH_DETAIL,
            false,
        ))
    }

    pub(crate) fn current_snapshot_admitted(
        &self,
        _permit: &ControllerMutationPermit,
    ) -> Result<TerminalSnapshot> {
        self.publish_current_snapshot()
    }

    pub(crate) fn current_snapshot_internal(&self) -> Result<TerminalSnapshot> {
        self.publish_current_snapshot()
    }

}

include!("state_core_snapshot.rs");

#[cfg(test)]
#[path = "../../unit_tests/state/state_text_bound_tests.rs"]
mod state_text_bound_tests;

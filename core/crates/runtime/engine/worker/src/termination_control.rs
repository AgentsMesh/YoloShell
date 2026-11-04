use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TerminationAuthority {
    Controller(AuthenticatedController),
    ReleasedRecovery,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TerminationIntent {
    reason: ExitReason,
    authority: TerminationAuthority,
}

impl WorkerState {
    pub(super) fn begin_controller_termination(&self, permit: &ControllerMutationPermit, reason: ExitReason) {
        self.record_termination(
            reason,
            TerminationAuthority::Controller(AuthenticatedController {
                generation: permit.generation,
            }),
        );
    }

    pub(super) fn begin_system_termination(&self, permit: &SystemMutationPermit, reason: ExitReason) {
        let alias_release = self.prepare_system_alias_release(permit);
        self.record_termination(reason, TerminationAuthority::System);
        self.commit_system_alias_release(permit, alias_release);
    }

    pub(super) fn begin_system_termination_blocking(self: &Arc<Self>, reason: ExitReason) {
        let permit = self.admit_system_mutation_blocking();
        self.begin_system_termination(&permit, reason);
        drop(permit);
    }

    pub(super) fn assume_released_recovery_termination_authority(&self, _permit: &SystemMutationPermit) {
        if let Some(intent) = lock_recover(&self.termination_intent).as_mut() {
            if matches!(intent.authority, TerminationAuthority::Controller(_)) {
                intent.authority = TerminationAuthority::ReleasedRecovery;
            }
        }
    }

    pub(super) fn upgrade_finalization_to_lost(&self, _proof: crate::namespace_lease::NamespaceOwnershipLoss) {
        debug_assert!(self.exit_finalization_started.load(Ordering::Acquire));
        // Finalization already revoked input under the admission gate. This proof
        // only strengthens its diagnostic reason and performs no PTY side effect.
        self.record_termination(ExitReason::Lost, TerminationAuthority::System);
    }

    pub(super) fn record_termination(&self, reason: ExitReason, authority: TerminationAuthority) {
        let mut current = lock_recover(&self.termination_intent);
        match current.as_mut() {
            None => {
                *current = Some(TerminationIntent { reason, authority });
            }
            Some(intent) => {
                intent.authority = merge_termination_authority(intent.authority, authority);
                intent.reason = merge_termination_reason(intent.reason, reason);
            }
        }
    }

    pub(super) fn termination_reason(&self) -> Option<ExitReason> {
        lock_recover(&self.termination_intent).map(|intent| intent.reason)
    }

    pub(super) fn has_termination_intent(&self) -> bool {
        lock_recover(&self.termination_intent).is_some()
    }

    pub(super) fn is_finalizing(&self) -> bool {
        self.child_waited.load(Ordering::Acquire) || self.has_termination_intent()
    }

    pub(super) fn notify_termination_shutdown_blocking(self: &Arc<Self>) {
        loop {
            let Some(authority) = lock_recover(&self.termination_intent)
                .as_ref()
                .map(|intent| intent.authority)
            else {
                return;
            };
            let permit = self.admit_system_mutation_blocking();
            if self.termination_authority() != Some(authority) {
                continue;
            }
            let generation_is_current = match authority {
                TerminationAuthority::Controller(controller) => {
                    self.controller_generation.load(Ordering::Acquire) == controller.generation.0
                }
                TerminationAuthority::ReleasedRecovery | TerminationAuthority::System => true,
            };
            if generation_is_current {
                self.commit_system_shutdown(&permit);
            }
            return;
        }
    }

    pub(super) fn attachment_clients_are_idle(&self) -> bool {
        self.attachment_clients.available_permits() == MAX_WORKER_ATTACHMENTS
    }

    pub(super) fn schedule_natural_exit_shutdown(self: &Arc<Self>) {
        if !self.runtime_serving.load(Ordering::Acquire) || self.has_termination_intent() {
            return;
        }
        if self
            .natural_exit_shutdown_scheduled
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }
        let state = Arc::clone(self);
        let spawned = std::thread::Builder::new()
            .name(NATURAL_EXIT_SHUTDOWN_THREAD.into())
            .spawn(move || {
                std::thread::sleep(NATURAL_EXIT_ATTACHMENT_GRACE);
                let deadline = std::time::Instant::now() + NATURAL_EXIT_ATTACHMENT_DRAIN_MAX;
                while !state.attachment_clients_are_idle() && std::time::Instant::now() < deadline {
                    std::thread::sleep(NATURAL_EXIT_ATTACHMENT_POLL);
                }
                if !state.attachment_clients_are_idle() {
                    tracing::warn!(
                        "natural exit worker shutdown exceeded its attachment drain deadline"
                    );
                }
                let permit = state.admit_system_mutation_blocking();
                state.commit_system_shutdown(&permit);
            });
        if let Err(error) = spawned {
            self.natural_exit_shutdown_scheduled
                .store(false, Ordering::Release);
            tracing::warn!(%error, "schedule natural exit worker shutdown");
        }
    }

    pub(super) fn termination_authority(&self) -> Option<TerminationAuthority> {
        lock_recover(&self.termination_intent)
            .as_ref()
            .map(|intent| intent.authority)
    }

    #[cfg(test)]
    pub(super) fn begin_termination_for_test(&self, reason: ExitReason) {
        self.record_termination(reason, TerminationAuthority::System);
    }
}

pub(super) fn merge_termination_reason(current: ExitReason, requested: ExitReason) -> ExitReason {
    match (current, requested) {
        (ExitReason::Lost, _) | (_, ExitReason::Lost) => ExitReason::Lost,
        (current, _) => current,
    }
}

pub(super) fn merge_termination_authority(
    current: TerminationAuthority,
    requested: TerminationAuthority,
) -> TerminationAuthority {
    match (current, requested) {
        (TerminationAuthority::System, _) | (_, TerminationAuthority::System) => {
            TerminationAuthority::System
        }
        (TerminationAuthority::ReleasedRecovery, _)
        | (_, TerminationAuthority::ReleasedRecovery) => TerminationAuthority::ReleasedRecovery,
        (_, controller) => controller,
    }
}

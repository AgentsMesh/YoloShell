use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ControllerGeneration(pub(super) u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WriterLeaseGeneration(pub(super) u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AuthenticatedController {
    pub(super) generation: ControllerGeneration,
}

pub(super) struct ControllerMutationPermit {
    _guard: tokio::sync::OwnedMutexGuard<()>,
    pub(super) generation: ControllerGeneration,
}

pub(super) struct WriterMutationPermit {
    _controller: ControllerMutationPermit,
    _generation: WriterLeaseGeneration,
}

pub(super) struct SystemMutationPermit {
    _guard: tokio::sync::OwnedMutexGuard<()>,
}

pub(super) struct AliasClosureMutationPermit {
    _guard: tokio::sync::OwnedMutexGuard<()>,
}

impl AuthenticatedController {
    #[cfg(test)]
    pub(super) fn new(generation: u64) -> Self {
        Self {
            generation: ControllerGeneration(generation),
        }
    }
}

impl WorkerState {
    pub(super) fn validate_controller(
        &self,
        controller: AuthenticatedController,
    ) -> std::result::Result<(), ErrorBody> {
        let requested = controller.generation.0;
        let current = self.controller_generation.load(Ordering::Acquire);
        let terminal = lock_recover(&self.exit).is_some();
        if requested == current && (self.custody_authorizes(controller.generation) || terminal) {
            return Ok(());
        }
        if requested == current {
            return Err(ErrorBody::conflict(
                secondary_custody_authority::CUSTODY_UNAVAILABLE_DETAIL,
            ));
        }
        Err(ErrorBody::conflict(format!(
            "controller generation {requested} was fenced by generation {current}"
        )))
    }

    pub(super) async fn admit_controller_mutation(
        self: &Arc<Self>,
        controller: AuthenticatedController,
    ) -> std::result::Result<ControllerMutationPermit, ErrorBody> {
        #[cfg(test)]
        self.control_test_hooks
            .controller_before_gate
            .pause_if_armed()
            .await;
        let guard = Arc::clone(&self.mutation_gate).lock_owned().await;
        self.validate_controller(controller)?;
        Ok(ControllerMutationPermit {
            _guard: guard,
            generation: controller.generation,
        })
    }

    pub(super) async fn admit_writer_mutation(
        self: &Arc<Self>,
        controller: AuthenticatedController,
        generation: u64,
        writer_capability: &WriterCapability,
    ) -> std::result::Result<WriterMutationPermit, ErrorBody> {
        let controller = self.admit_controller_mutation(controller).await?;
        self.ensure_ready()?;
        self.ensure_process_running()?;
        self.validate_lease_admitted(generation, writer_capability)?;
        Ok(WriterMutationPermit {
            _controller: controller,
            _generation: WriterLeaseGeneration(generation),
        })
    }

    pub(super) fn ensure_snapshot_or_attachment_allowed(&self) -> std::result::Result<(), ErrorBody> {
        if lock_recover(&self.exit).is_some() {
            return Ok(());
        }
        if self.is_finalizing() {
            return self.ensure_process_running();
        }
        Ok(())
    }

    pub(super) async fn admit_system_mutation(self: &Arc<Self>) -> SystemMutationPermit {
        SystemMutationPermit {
            _guard: Arc::clone(&self.mutation_gate).lock_owned().await,
        }
    }

    pub(super) async fn admit_alias_closure_mutation(
        self: &Arc<Self>,
        _authority: &pty_runtime_alias_closure_task::DurableAliasClosure,
    ) -> AliasClosureMutationPermit {
        #[cfg(test)]
        let mut admission = Box::pin(Arc::clone(&self.mutation_gate).lock_owned());
        #[cfg(test)]
        let guard = std::future::poll_fn(|context| {
            use std::future::Future;
            match admission.as_mut().poll(context) {
                std::task::Poll::Ready(guard) => std::task::Poll::Ready(guard),
                std::task::Poll::Pending => {
                    self.control_test_hooks
                        .alias_closure_waiter_queued
                        .mark_reached();
                    std::task::Poll::Pending
                }
            }
        })
        .await;
        #[cfg(not(test))]
        let admission = Box::pin(Arc::clone(&self.mutation_gate).lock_owned());
        #[cfg(not(test))]
        let guard = admission.await;
        AliasClosureMutationPermit { _guard: guard }
    }

    pub(super) fn admit_system_mutation_blocking(self: &Arc<Self>) -> SystemMutationPermit {
        #[cfg(test)]
        self.control_test_hooks
            .system_mutation_before_gate
            .mark_reached();
        SystemMutationPermit {
            _guard: Arc::clone(&self.mutation_gate).blocking_lock_owned(),
        }
    }

    pub(super) fn commit_controller_shutdown(&self, _permit: &ControllerMutationPermit) {
        self.commit_shutdown();
    }

    pub(super) fn commit_system_shutdown(&self, _permit: &SystemMutationPermit) {
        self.commit_shutdown();
    }

    pub(super) fn commit_shutdown(&self) {
        self.commit_response_shutdown();
    }

    pub(super) fn schedule_controller_shutdown(self: &Arc<Self>, controller: AuthenticatedController) {
        let state = Arc::clone(self);
        tokio::spawn(async move {
            tokio::time::sleep(EXPLICIT_EXIT_SHUTDOWN_DELAY).await;
            if let Ok(permit) = state.admit_controller_mutation(controller).await {
                state.commit_controller_shutdown(&permit);
            }
        });
    }

    #[cfg(test)]
    pub(super) fn current_authenticated_controller(&self) -> AuthenticatedController {
        AuthenticatedController::new(self.controller_generation.load(Ordering::Acquire))
    }
}

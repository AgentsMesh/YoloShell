impl IngressSupervisorInner {
    fn lock_state(&self) -> std::sync::MutexGuard<'_, IngressState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(super) fn reserve(
        &self,
        operation: Arc<IngressOperation>,
    ) -> std::result::Result<(), Arc<IngressOperation>> {
        let mut state = self.lock_state();
        if state.operation().is_some_and(|active| active.is_complete()) {
            *state = IngressState::Idle;
        }
        if !matches!(*state, IngressState::Idle) {
            return Err(operation);
        }
        *state = IngressState::Starting(operation);
        Ok(())
    }

    pub(super) fn install_listener(
        &self,
        operation_identity: &Arc<()>,
        listener_identity: Arc<()>,
        abort: tokio::task::AbortHandle,
        completion: super::ingress_listener_completion::IngressListenerCompletion,
        router: Arc<super::router::CodexSessionRouter>,
    ) -> bool {
        let mut state = self.lock_state();
        let operation = match &*state {
            IngressState::Starting(operation) | IngressState::Restarting(operation) => {
                Arc::clone(operation)
            }
            _ => return false,
        };
        if !operation.matches(operation_identity) {
            return false;
        }
        let mut listener = crate::worker::lock_recover(&self.listener);
        if listener.is_some() {
            return false;
        }
        listener.replace(IngressListener {
            identity: listener_identity,
            abort,
            completion,
        });
        *state = IngressState::Running(operation);
        crate::worker::lock_recover(&self.projection_router).replace(router);
        #[cfg(test)]
        self.test_control.record_publication();
        true
    }

    pub(super) fn permits_bind(&self, operation_identity: &Arc<()>) -> bool {
        match &*self.lock_state() {
            IngressState::Starting(operation) | IngressState::Restarting(operation) => {
                operation.matches(operation_identity)
            }
            _ => false,
        }
    }

    pub(super) fn finish_listener(
        &self,
        operation_identity: &Arc<()>,
        listener_identity: &Arc<()>,
    ) -> bool {
        let mut state = self.lock_state();
        let Some(operation) = state.operation().cloned() else {
            return false;
        };
        if !operation.matches(operation_identity) {
            return false;
        }
        let mut listener = crate::worker::lock_recover(&self.listener);
        if !listener
            .as_ref()
            .is_some_and(|active| Arc::ptr_eq(&active.identity, listener_identity))
        {
            return false;
        }
        listener.take();
        crate::worker::lock_recover(&self.projection_router).take();
        if matches!(*state, IngressState::Running(_)) {
            *state = IngressState::Restarting(operation);
            true
        } else {
            false
        }
    }

    pub(super) fn mark_finished(&self, identity: &Arc<()>) {
        let mut state = self.lock_state();
        let Some(operation) = state.operation().cloned() else {
            return;
        };
        if operation.matches(identity) {
            *state = IngressState::Finished(operation);
            if let Some(listener) = crate::worker::lock_recover(&self.listener).as_ref() {
                listener.abort.abort();
            }
            crate::worker::lock_recover(&self.projection_router).take();
        }
    }

    fn abort_listener(
        &self,
        operation: &Arc<IngressOperation>,
    ) -> Option<super::ingress_listener_completion::IngressListenerCompletion> {
        let state = self.lock_state();
        if !state
            .operation()
            .is_some_and(|active| Arc::ptr_eq(active, operation))
        {
            return None;
        }
        let listener = crate::worker::lock_recover(&self.listener);
        let listener = listener.as_ref()?;
        listener.abort.abort();
        Some(listener.completion.clone())
    }

    fn begin_shutdown(&self) -> Option<Arc<IngressOperation>> {
        let mut state = self.lock_state();
        let operation = state.operation()?.clone();
        *state = IngressState::Stopping(Arc::clone(&operation));
        Some(operation)
    }

    fn clear(&self, completed: &Arc<IngressOperation>) {
        let mut state = self.lock_state();
        if state
            .operation()
            .is_some_and(|active| Arc::ptr_eq(active, completed))
        {
            *state = IngressState::Idle;
            if let Some(listener) = crate::worker::lock_recover(&self.listener).take() {
                listener.abort.abort();
            }
            crate::worker::lock_recover(&self.projection_router).take();
        }
    }

    #[cfg(test)]
    async fn wait_start_gate(&self) {
        let gate = self
            .start_gate
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(gate) = gate {
            gate.block().await;
        }
    }

    #[cfg(not(test))]
    async fn wait_start_gate(&self) {}
}

impl IngressState {
    fn operation(&self) -> Option<&Arc<IngressOperation>> {
        match self {
            Self::Idle => None,
            Self::Starting(operation)
            | Self::Running(operation)
            | Self::Restarting(operation)
            | Self::Stopping(operation)
            | Self::Finished(operation) => Some(operation),
        }
    }
}

#[cfg(test)]
impl IngressStartGate {
    pub(in crate::worker::codex_hook) async fn wait_until_entered(&self) {
        self.entered.notified().await;
    }

    pub(in crate::worker::codex_hook) fn release(&self) {
        self.released.notify_one();
    }

    async fn block(&self) {
        self.entered.notify_one();
        self.released.notified().await;
    }
}

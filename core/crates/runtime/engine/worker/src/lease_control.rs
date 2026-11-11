use super::*;

impl WorkerState {
    pub(super) async fn acquire_lease(
        self: &Arc<Self>,
        controller: AuthenticatedController,
        request: &LeaseRequest,
        takeover: bool,
    ) -> std::result::Result<WriterLeaseGrant, ErrorBody> {
        let _permit = self.admit_controller_mutation(controller).await?;
        self.ensure_ready()?;
        self.ensure_process_running()?;
        let mut transitions = Vec::new();
        let result = lock_recover(&self.lease).acquire(request, takeover, &mut |transition| {
            transitions.push(transition);
        });
        self.emit_writer_lease_transitions(transitions);
        result
    }

    pub(super) async fn renew_lease(
        self: &Arc<Self>,
        controller: AuthenticatedController,
        request: &LeaseRequest,
    ) -> std::result::Result<WriterLeaseGrant, ErrorBody> {
        let _permit = self.admit_controller_mutation(controller).await?;
        self.ensure_ready()?;
        self.ensure_process_running()?;
        let mut transitions = Vec::new();
        let result = lock_recover(&self.lease).renew(request, &mut |transition| {
            transitions.push(transition);
        });
        self.emit_writer_lease_transitions(transitions);
        result
    }

    pub(super) async fn release_lease(
        self: &Arc<Self>,
        controller: AuthenticatedController,
        request: &LeaseRequest,
    ) -> std::result::Result<(), ErrorBody> {
        let _permit = self.admit_controller_mutation(controller).await?;
        self.ensure_process_running()?;
        let mut transitions = Vec::new();
        let result = lock_recover(&self.lease).release(request, &mut |transition| {
            transitions.push(transition);
        });
        if result.is_ok() {
            lock_recover(&self.delivered_inputs).clear_capability(&request.writer_capability);
        }
        self.emit_writer_lease_transitions(transitions);
        result
    }

    pub(super) async fn get_writer_lease(
        self: &Arc<Self>,
        controller: AuthenticatedController,
    ) -> std::result::Result<Option<WriterLease>, ErrorBody> {
        let _permit = self.admit_controller_mutation(controller).await?;
        self.ensure_process_running()?;
        let mut transitions = Vec::new();
        let lease = lock_recover(&self.lease).snapshot(&mut |transition| {
            transitions.push(transition);
        });
        self.emit_writer_lease_transitions(transitions);
        Ok(lease)
    }

    pub(super) fn validate_lease_admitted(
        &self,
        generation: u64,
        writer_capability: &WriterCapability,
    ) -> std::result::Result<(), ErrorBody> {
        let mut transitions = Vec::new();
        let result = lock_recover(&self.lease).validate_authorization(
            generation,
            writer_capability,
            &mut |transition| transitions.push(transition),
        );
        self.emit_writer_lease_transitions(transitions);
        result
    }

    pub(super) fn revoke_writer_lease_admitted(&self, _permit: &SystemMutationPermit, reason: LeaseReason) {
        let mut transitions = Vec::new();
        lock_recover(&self.lease).revoke(reason, &mut |transition| {
            transitions.push(transition);
        });
        self.emit_writer_lease_transitions(transitions);
    }

    pub(super) fn emit_writer_lease_transitions(&self, transitions: Vec<LeaseTransition>) {
        for transition in transitions {
            self.emit_writer_lease_transition(transition);
        }
    }

    pub(super) fn emit_writer_lease_transition(&self, transition: LeaseTransition) {
        let event_type = match transition.kind {
            LeaseTransitionKind::Acquired => event::WRITER_LEASE_ACQUIRED,
            LeaseTransitionKind::Revoked => event::WRITER_LEASE_REVOKED,
            LeaseTransitionKind::Expired => event::WRITER_LEASE_EXPIRED,
        };
        let payload = serde_json::to_value(LeaseEventPayload::from(transition))
            .expect("writer lease event must serialize");
        let _ = self.events.send(WorkerEvent::WriterLease {
            event_type,
            payload,
        });
    }

    pub(super) fn ensure_process_running(&self) -> std::result::Result<(), ErrorBody> {
        if self.has_termination_intent()
            || self.child_waited.load(Ordering::Acquire)
            || lock_recover(&self.exit).is_some()
        {
            return Err(ErrorBody::new(
                lease_error_code::SESSION_NOT_RUNNING,
                "session process has exited",
                false,
            ));
        }
        Ok(())
    }

    pub(super) fn ensure_termination_allowed(&self) -> std::result::Result<(), ErrorBody> {
        if lock_recover(&self.exit).is_some() {
            return Ok(());
        }
        self.ensure_process_running()
    }

    pub(super) fn ensure_ready(&self) -> std::result::Result<(), ErrorBody> {
        if self.ready.load(Ordering::Acquire) {
            return Ok(());
        }
        Err(ErrorBody::conflict(WORKER_NOT_READY_DETAIL))
    }
}

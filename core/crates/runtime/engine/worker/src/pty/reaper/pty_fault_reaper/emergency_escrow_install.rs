use super::*;

impl EmergencyEscrowProducer {
    pub(crate) fn install(
        mut self,
        custody: PtyOwnershipCustody,
        completion: Option<FaultReapCompletion>,
        failure: Option<anyhow::Error>,
        panic: Option<Box<dyn Any + Send>>,
        retired_reservation: Option<FaultReaperReservation>,
    ) {
        let Some(mut reap) = self.control.take() else {
            std::mem::forget((
                self,
                custody,
                completion,
                failure,
                panic,
                retired_reservation,
            ));
            return;
        };
        let Some(default_completion) = self.completion.take() else {
            std::mem::forget((
                self,
                reap,
                custody,
                completion,
                failure,
                panic,
                retired_reservation,
            ));
            return;
        };
        let (completion, retired_completion) = match completion {
            Some(completion) => (completion, Some(default_completion)),
            None => (default_completion, None),
        };
        let Some(slot) = self.slot.take() else {
            std::mem::forget((
                self,
                reap,
                custody,
                completion,
                failure,
                panic,
                retired_reservation,
            ));
            return;
        };
        let slot_pointer = Arc::as_ptr(&slot);
        reap.handoff = Some(OwnershipFaultHandoff::new(custody, completion));
        reap.failure = failure;
        let envelope = EmergencyEnvelope {
            reap,
            panic,
            _retired_completion: retired_completion,
            _retired_reservation: retired_reservation,
            _producer_slot: slot,
        };
        // SAFETY: the Arc now held by the envelope keeps slot_pointer live. This
        // consuming producer is the only writer and publishes after initialization.
        unsafe {
            (*(*slot_pointer).envelope.get()).write(envelope);
            (*slot_pointer)
                .phase
                .store(EmergencyPhase::Ready as u8, Ordering::Release);
            (*slot_pointer).dispatcher.unpark();
            #[cfg(test)]
            (*slot_pointer).test_hooks.wait_before_install_return();
        }
    }
}

impl FaultReaperReservation {
    pub(crate) fn install_emergency_after_drop(
        mut self,
        custody: PtyOwnershipCustody,
        completion: Option<FaultReapCompletion>,
        failure: Option<anyhow::Error>,
        panic: Option<Box<dyn Any + Send>>,
    ) {
        let Some(emergency) = self.emergency.take() else {
            std::mem::forget((self, custody, completion, failure, panic));
            return;
        };
        emergency.install(custody, completion, failure, panic, Some(self));
    }
}

#[cfg(test)]
impl EmergencyEscrowProducer {
    pub(crate) fn install_after_panic(
        self,
        custody: PtyOwnershipCustody,
        panic: Box<dyn Any + Send>,
    ) {
        self.install(custody, None, None, Some(panic), None);
    }
}

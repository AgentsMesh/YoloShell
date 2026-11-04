use super::PtyOwnership;

impl Drop for PtyOwnership {
    fn drop(&mut self) {
        if !self.armed || self.reap_evidence.is_some() {
            return;
        }
        self.armed = false;
        #[cfg(test)]
        let payload = self.emergency_payload.take();
        #[cfg(not(test))]
        let payload = None;
        self.escrow_retained_custody(payload);
    }
}

impl PtyOwnership {
    pub(crate) fn escrow_retained_custody(
        &mut self,
        payload: Option<Box<dyn std::any::Any + Send>>,
    ) {
        let Some(mut armed) = self.armed_ownership.take() else {
            std::mem::forget(payload);
            return;
        };
        let Some(custody) = armed.custody.take() else {
            std::mem::forget((armed, payload));
            return;
        };
        let completion = armed.fault_completion.take();
        let failure = armed.fault.take();
        armed
            .fault_reaper
            .install_emergency_after_drop(custody, completion, failure, payload);
    }

    #[cfg(test)]
    pub(crate) fn inject_emergency_payload_for_test(
        &mut self,
        payload: Box<dyn std::any::Any + Send>,
    ) {
        assert!(self.emergency_payload.replace(payload).is_none());
    }
}

pub(super) struct HostResponseTicket {
    state: Option<Arc<HostRequestDrainState>>,
}

impl HostResponseTicket {
    pub(super) fn publish(mut self) {
        self.resolve();
    }

    fn resolve(&mut self) {
        let Some(state) = self.state.take() else {
            return;
        };
        let mut responses = super::lock_recover(&state.responses);
        responses.pending = responses
            .pending
            .checked_sub(1)
            .expect(HOST_RESPONSE_DRAIN_UNDERFLOW);
        drop(responses);
        state.responses_changed.notify_waiters();
    }
}

impl Drop for HostResponseTicket {
    fn drop(&mut self) {
        self.resolve();
    }
}

impl CodexOutboxState {
    pub fn acknowledged_sequence(&self) -> u64 {
        self.acknowledged_sequence
    }

    pub fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    pub fn owner(&self) -> Option<&HookIdentity> {
        self.owner.as_ref()
    }

    pub fn pending_head(&self) -> Option<&HookEnvelope> {
        self.pending.front()
    }

    pub fn retained_events(&self) -> impl Iterator<Item = &HookEnvelope> {
        self.acknowledged_events.iter().chain(self.pending.iter())
    }

    pub fn is_acknowledged(&self, event: &HookEnvelope) -> bool {
        self.acknowledged_events
            .iter()
            .any(|retained| retained == event)
    }

    pub fn durable_rejections(
        &self,
    ) -> impl Iterator<Item = &super::outbox_rejection::CodexDurableRejection> {
        self.durable_rejections.iter()
    }
}

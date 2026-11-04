impl SecondaryCustodyLease {
    pub(super) async fn panic_monitor(&self) {
        self.monitor_commands
            .send(SecondaryCustodyCommand::Panic)
            .await
            .expect(MONITOR_UNAVAILABLE_DETAIL);
    }

    pub(super) fn settlement_is_active(&self) -> bool {
        self.settlement_active.load(Ordering::Acquire)
    }

    pub(super) fn monitor_is_pending(&self) -> bool {
        super::super::lock_recover(&self.monitor_receiver).is_some()
    }
}

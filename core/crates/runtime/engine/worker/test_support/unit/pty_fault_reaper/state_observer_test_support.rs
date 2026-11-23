impl FaultReap {
    pub(super) fn publish_state_for_test(&self, output_fence_reached: Option<bool>) {
        self.publish_state_with_failure_for_test(output_fence_reached, self.failure.is_some());
    }

    fn publish_state_with_failure_for_test(
        &self,
        output_fence_reached: Option<bool>,
        failure_present: bool,
    ) {
        let custody = self.custody();
        self.state_observer
            .publish_snapshot(FaultReapStateSnapshot {
                custody: custody
                    .processes
                    .as_ref()
                    .map(|custody| custody.state_snapshot_for_test()),
                wait_authority_present: custody.authority.is_some(),
                wait_authority_lost: self.lost_wait_custody.is_some(),
                root_exit_proof_present: custody.root_exit_proof.is_some(),
                root_waited: custody
                    .authority
                    .as_ref()
                    .is_some_and(crate::process::RootWaitAuthority::is_waited),
                root_gone_without_wait: self.root_gone_without_wait,
                observer_only: self.is_observer_only(),
                completion_published: self.completion().is_published(),
                failure_present,
                output_fence_reached,
            });
    }

    pub(super) fn publish_terminal_state_for_test(&self, failure_present: bool) {
        self.publish_state_with_failure_for_test(None, failure_present);
    }
}

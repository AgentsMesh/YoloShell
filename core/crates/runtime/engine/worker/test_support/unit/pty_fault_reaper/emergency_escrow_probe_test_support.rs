impl EmergencyEscrowProbe {
    pub(super) fn fail_next_owner_spawn(&self) {
        self.configure_owner_spawn_fault(InjectedOwnerSpawnFault::Error, true);
    }

    pub(super) fn panic_next_owner_spawn(&self) {
        self.configure_owner_spawn_fault(InjectedOwnerSpawnFault::Panic, true);
    }

    pub(super) fn fail_owner_spawns(&self, count: usize) {
        let mut state = lock(&self.shared.state);
        assert_eq!(state.owner_spawn_failures_remaining, usize::default());
        state.owner_spawn_fault = Some(InjectedOwnerSpawnFault::Error);
        state.owner_spawn_failures_remaining = count;
        state.block_owner_retry = false;
        state.allow_owner_retry = true;
    }

    fn configure_owner_spawn_fault(&self, fault: InjectedOwnerSpawnFault, block_retry: bool) {
        let mut state = lock(&self.shared.state);
        assert_eq!(state.owner_spawn_failures_remaining, usize::default());
        state.owner_spawn_fault = Some(fault);
        state.owner_spawn_failures_remaining = SINGLE_OWNER_SPAWN_FAILURE;
        state.block_owner_retry = block_retry;
        state.allow_owner_retry = false;
    }

    pub(super) fn wait_for_owner_spawn_failure(&self) {
        self.wait_for_owner_spawn_failures(SINGLE_OWNER_SPAWN_FAILURE);
    }

    pub(super) fn wait_for_owner_spawn_failures(&self, count: usize) {
        wait_for_state(&self.shared, |state| {
            state.owner_spawn_failures_observed >= count
        });
        assert!(
            lock(&self.shared.state).owner_spawn_failures_observed >= count,
            "{OWNER_FAILURE_WAIT_DETAIL}"
        );
    }

    pub(super) fn allow_owner_spawn_retry(&self) {
        lock(&self.shared.state).allow_owner_retry = true;
        self.shared.changed.notify_all();
    }

    pub(super) fn block_install_return(&self) {
        let mut state = lock(&self.shared.state);
        state.install_publication_observed = false;
        state.allow_install_return = false;
        self.shared
            .block_install_return
            .store(true, Ordering::Release);
    }

    pub(super) fn wait_for_install_publication(&self) {
        wait_for_state(&self.shared, |state| state.install_publication_observed);
        assert!(lock(&self.shared.state).install_publication_observed);
    }

    pub(super) fn allow_install_return(&self) {
        lock(&self.shared.state).allow_install_return = true;
        self.shared.changed.notify_all();
    }

    pub(super) fn wait_for_drive_schedule(&self) -> bool {
        wait_for_state(&self.shared, |state| state.drive_schedule_prompt.is_some());
        lock(&self.shared.state)
            .drive_schedule_prompt
            .expect(DRIVE_SCHEDULE_WAIT_DETAIL)
    }

    pub(super) fn phase(&self) -> Option<EmergencyPhase> {
        self.slot.upgrade().map(|slot| slot.phase())
    }

    pub(super) fn strong_count(&self) -> usize {
        self.slot.strong_count()
    }

    pub(super) fn wait_for_slot_drop(&self) {
        wait_for_state(&self.shared, |state| state.slot_dropped);
        assert!(
            lock(&self.shared.state).slot_dropped,
            "{SLOT_DROP_WAIT_DETAIL}"
        );
    }
}

impl Drop for EmergencyEscrowProbe {
    fn drop(&mut self) {
        self.allow_owner_spawn_retry();
        self.allow_install_return();
    }
}

fn wait_for_state(shared: &HookShared, predicate: impl Fn(&HookState) -> bool) {
    let state = lock(&shared.state);
    let _ = shared
        .changed
        .wait_timeout_while(state, HOOK_WAIT_TIMEOUT, |state| !predicate(state))
        .unwrap_or_else(std::sync::PoisonError::into_inner);
}

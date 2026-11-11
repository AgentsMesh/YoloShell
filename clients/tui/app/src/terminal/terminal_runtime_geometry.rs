// Included by `terminal_runtime`; extends the same `State` impl module.

impl State {
    pub(crate) fn renewal_pending(&self, address: TerminalAddress) -> bool {
        self.runtime_facts(address)
            .is_some_and(|facts| facts.bound.connection.renewal_pending)
    }

    pub(crate) fn mark_renewal_pending(&mut self, address: TerminalAddress) {
        self.runtime_facts_mut(address)
            .bound
            .connection
            .renewal_pending = true;
    }

    pub(crate) fn clear_renewal_pending(&mut self, address: TerminalAddress) {
        if let Some(facts) = self.terminal_runtime.get_mut(&address) {
            facts.bound.connection.renewal_pending = false;
        }
        self.prune_runtime_facts(address);
    }

    pub(crate) fn last_size(&self, address: TerminalAddress) -> Option<TerminalSize> {
        self.runtime_facts(address)
            .and_then(|facts| facts.bound.writer_geometry.last_size)
    }

    pub(crate) fn set_last_size(&mut self, address: TerminalAddress, size: TerminalSize) {
        self.runtime_facts_mut(address)
            .bound
            .writer_geometry
            .last_size = Some(size);
    }

    pub(crate) fn clear_last_size(&mut self, address: TerminalAddress) {
        if let Some(facts) = self.terminal_runtime.get_mut(&address) {
            facts.bound.writer_geometry.last_size = None;
        }
        self.prune_runtime_facts(address);
    }

    pub(crate) fn is_resync(&self, address: TerminalAddress) -> bool {
        self.runtime_facts(address)
            .is_some_and(|facts| facts.bound.resync)
    }

    pub(crate) fn mark_resync(&mut self, address: TerminalAddress) {
        self.runtime_facts_mut(address).bound.resync = true;
    }

    pub(crate) fn clear_resync(&mut self, address: TerminalAddress) {
        if let Some(facts) = self.terminal_runtime.get_mut(&address) {
            facts.bound.resync = false;
        }
        self.prune_runtime_facts(address);
    }

    pub(crate) fn take_resync(&mut self) -> BTreeSet<TerminalAddress> {
        let addresses = self
            .terminal_runtime
            .iter()
            .filter(|(_, facts)| facts.bound.resync)
            .map(|(address, _)| *address)
            .collect::<BTreeSet<_>>();
        for address in &addresses {
            self.clear_resync(*address);
        }
        addresses
    }

    pub(crate) fn is_geometry_syncing(&self, address: TerminalAddress) -> bool {
        self.runtime_facts(address)
            .is_some_and(|facts| facts.bound.writer_geometry.geometry_syncing)
    }

    pub(crate) fn mark_geometry_syncing(&mut self, address: TerminalAddress) {
        self.runtime_facts_mut(address)
            .bound
            .writer_geometry
            .geometry_syncing = true;
    }

    pub(crate) fn clear_geometry_syncing(&mut self, address: TerminalAddress) {
        if let Some(facts) = self.terminal_runtime.get_mut(&address) {
            facts.bound.writer_geometry.geometry_syncing = false;
        }
        self.prune_runtime_facts(address);
    }

    pub(crate) fn is_resize_pending(&self, address: TerminalAddress) -> bool {
        self.runtime_facts(address)
            .is_some_and(|facts| facts.bound.writer_geometry.resize_pending)
    }

    pub(crate) fn mark_resize_pending(&mut self, address: TerminalAddress) {
        self.runtime_facts_mut(address)
            .bound
            .writer_geometry
            .resize_pending = true;
    }

    pub(crate) fn clear_resize_pending(&mut self, address: TerminalAddress) {
        if let Some(facts) = self.terminal_runtime.get_mut(&address) {
            facts.bound.writer_geometry.resize_pending = false;
        }
        self.prune_runtime_facts(address);
    }

    pub(crate) fn resize_in_flight(&self, address: TerminalAddress) -> Option<PendingResize> {
        self.runtime_facts(address)
            .and_then(|facts| facts.bound.writer_geometry.resize_in_flight)
    }

    pub(crate) fn set_resize_in_flight(
        &mut self,
        address: TerminalAddress,
        pending: PendingResize,
    ) {
        self.runtime_facts_mut(address)
            .bound
            .writer_geometry
            .resize_in_flight = Some(pending);
    }

    pub(crate) fn clear_resize_in_flight(&mut self, address: TerminalAddress) {
        if let Some(facts) = self.terminal_runtime.get_mut(&address) {
            facts.bound.writer_geometry.resize_in_flight = None;
        }
        self.prune_runtime_facts(address);
    }

    pub(crate) fn codex_open_attempt(&self, address: TerminalAddress) -> Option<Instant> {
        self.runtime_facts(address)
            .and_then(|facts| facts.bound.connection.codex_open_attempt)
    }

    pub(crate) fn set_codex_open_attempt(&mut self, address: TerminalAddress, instant: Instant) {
        self.runtime_facts_mut(address)
            .bound
            .connection
            .codex_open_attempt = Some(instant);
    }

    pub(crate) fn clear_codex_open_attempt(&mut self, address: TerminalAddress) {
        if let Some(facts) = self.terminal_runtime.get_mut(&address) {
            facts.bound.connection.codex_open_attempt = None;
        }
        self.prune_runtime_facts(address);
    }
}

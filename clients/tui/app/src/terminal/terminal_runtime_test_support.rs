// Included by `terminal_runtime` under cfg(test); extends the same module.

impl State {
    #[cfg(test)]
    pub(crate) fn has_renewals_pending(&self) -> bool {
        self.terminal_runtime
            .values()
            .any(|facts| facts.bound.connection.renewal_pending)
    }

    #[cfg(test)]
    pub(crate) fn clear_all_last_sizes(&mut self) {
        for facts in self.terminal_runtime.values_mut() {
            facts.bound.writer_geometry.last_size = None;
        }
        self.terminal_runtime.retain(|_, facts| !facts.is_empty());
    }

    #[cfg(test)]
    pub(crate) fn last_size_snapshot(&self) -> BTreeMap<TerminalAddress, TerminalSize> {
        self.terminal_runtime
            .iter()
            .filter_map(|(address, facts)| {
                facts
                    .bound
                    .writer_geometry
                    .last_size
                    .map(|size| (*address, size))
            })
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn has_resync(&self) -> bool {
        self.terminal_runtime
            .values()
            .any(|facts| facts.bound.resync)
    }

    #[cfg(test)]
    pub(crate) fn clear_all_resync(&mut self) {
        for facts in self.terminal_runtime.values_mut() {
            facts.bound.resync = false;
        }
        self.terminal_runtime.retain(|_, facts| !facts.is_empty());
    }

    #[cfg(test)]
    pub(crate) fn clear_all_resize_pending(&mut self) {
        for facts in self.terminal_runtime.values_mut() {
            facts.bound.writer_geometry.resize_pending = false;
        }
        self.terminal_runtime.retain(|_, facts| !facts.is_empty());
    }

    #[cfg(test)]
    pub(crate) fn clear_all_resize_in_flight(&mut self) {
        for facts in self.terminal_runtime.values_mut() {
            facts.bound.writer_geometry.resize_in_flight = None;
        }
        self.terminal_runtime.retain(|_, facts| !facts.is_empty());
    }

    #[cfg(test)]
    pub(crate) fn observer_generation_snapshot(&self) -> BTreeMap<TerminalAddress, Uuid> {
        self.terminal_runtime
            .iter()
            .filter_map(|(address, facts)| {
                facts
                    .bound
                    .observer_generation
                    .map(|generation| (*address, generation))
            })
            .collect()
    }
}

impl WorkerState {
    pub(crate) fn runtime_capabilities(&self) -> Vec<String> {
        let mut capabilities = self
            .pty_owner_handle()
            .map(|owner| owner.runtime_capabilities())
            .unwrap_or_default();
        // Protocol support is stable after initialization. Actual source/history
        // availability is checked by observation and again at effect admission;
        // a busy journal must not revoke support or block hello on filesystem I/O.
        if self.native_control_initialized.load(Ordering::Acquire) {
            capabilities.push(crate::protocol::capability::NATIVE_AGENT_CONTROL.into());
        }
        capabilities
    }

    pub(crate) fn hello(&self) -> WorkerHello {
        // Do not retain the record mutex while capability verification reads it.
        let runtime = lock_recover(&self.record).clone();
        WorkerHello {
            runtime,
            capabilities: self.runtime_capabilities(),
        }
    }
}

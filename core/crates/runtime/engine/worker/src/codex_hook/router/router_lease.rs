use super::*;

impl CodexRouteLease {
    pub(super) fn new(
        entry: Arc<CodexRouteEntry>,
        router: Arc<RouterEntries>,
        replay: bool,
    ) -> Self {
        Self {
            entry,
            router,
            replay,
            released: false,
        }
    }
}

impl CodexRetirementPermit {
    pub(super) fn new(entry: Arc<CodexRouteEntry>, router: Arc<RouterEntries>) -> Self {
        Self {
            entry,
            router,
            released: false,
        }
    }

    pub(in crate::worker::codex_hook) fn entry(&self) -> &Arc<CodexRouteEntry> {
        &self.entry
    }

    pub(in crate::worker::codex_hook) fn verify(&self) -> Result<(), CodexRouterError> {
        self.entry.admission().verify()?;
        self.router
            .verify_retirement_permit(self.entry.key(), &self.entry)?;
        self.entry.admission().verify()
    }

    pub(in crate::worker::codex_hook) fn commit(mut self) -> Result<(), CodexRouterError> {
        self.entry.admission().verify()?;
        self.router
            .commit_retirement(self.entry.key(), &self.entry)?;
        self.released = true;
        Ok(())
    }

    fn release(&mut self) -> Result<(), CodexRouterError> {
        if self.released {
            return Ok(());
        }
        self.router
            .release_retirement_permit(self.entry.key(), &self.entry)?;
        self.released = true;
        Ok(())
    }
}

impl Drop for CodexRetirementPermit {
    fn drop(&mut self) {
        let _ = self.release();
    }
}

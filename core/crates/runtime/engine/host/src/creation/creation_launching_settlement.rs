use super::*;

impl Host {
    /// Settle crash-window creations whose launch evidence is already durable:
    /// a Host exit before the phase write must not strand the terminal.
    pub fn settle_launching_creations(&self) -> anyhow::Result<()> {
        let path = self.paths.home().join(CREATION_LEDGER_FILE_NAME);
        let launching: Vec<CreationBinding> = lock_recover(&self.creation_capabilities)
            .bindings
            .iter()
            .filter(|binding| binding.phase == CreationPhase::Launching)
            .cloned()
            .collect();
        for binding in launching {
            let Some(planned) = binding.launch.as_ref() else {
                continue;
            };
            if self.creation_runtime_evidence(&binding, planned) != CreationRuntimeEvidence::Ready {
                continue;
            }
            lock_recover(&self.creation_capabilities)
                .set_active_phase(&path, binding.result, CreationPhase::Ready)
                .map_err(|error| anyhow::anyhow!("{}", error.message))?;
        }
        Ok(())
    }
}

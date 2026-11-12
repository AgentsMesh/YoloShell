use super::build_gate::RouteBuildGate;
use super::install_state::InstallDecision;
use super::state::RouterEntries;
use super::{
    CodexHookRuntime, CodexRouteEntry, CodexRouteKey, CodexRouteLease, CodexRouterError,
    CodexSessionRouter, DurableCodexAdmission,
};
use crate::agent::hooks::HookIdentity;
use crate::codex_hooks::CodexAdmissionRecord;
use std::sync::Arc;

struct BuildReservation {
    entries: Arc<RouterEntries>,
    key: CodexRouteKey,
    gate: Arc<RouteBuildGate>,
    finished: bool,
}

impl CodexSessionRouter {
    pub(in crate::worker::codex_hook) fn install_or_replay<Builder>(
        &self,
        admission: DurableCodexAdmission,
        builder: Builder,
    ) -> Result<CodexRouteLease, CodexRouterError>
    where
        Builder: FnOnce(&CodexAdmissionRecord) -> Result<Arc<CodexHookRuntime>, CodexRouterError>,
    {
        admission.verify()?;
        let identity = admission.record().request().identity.clone();
        self.validate_scope(&identity)?;
        self.validate_admission_path(&identity, &admission)?;
        let key = CodexRouteKey::from_identity(&identity);
        let mut builder = Some(builder);
        loop {
            match self
                .inner
                .prepare_install(key, &admission, self.capacity_limit)?
            {
                InstallDecision::Replay(entry) => {
                    let lease = CodexRouteLease::new(entry, Arc::clone(&self.inner), true);
                    return verify_replay(lease, &admission);
                }
                InstallDecision::Wait(gate) => gate.wait()?,
                InstallDecision::Build(gate) => {
                    let reservation =
                        BuildReservation::new(Arc::clone(&self.inner), key, Arc::clone(&gate));
                    let built = self.build_entry(
                        key,
                        admission,
                        &identity,
                        builder.take().ok_or(CodexRouterError::Conflict)?,
                    );
                    let entry = match built {
                        Ok(entry) => reservation.publish(entry)?,
                        Err(error) => return Err(reservation.fail(error)),
                    };
                    return Ok(CodexRouteLease::new(entry, Arc::clone(&self.inner), false));
                }
            }
        }
    }

    fn build_entry<Builder>(
        &self,
        key: CodexRouteKey,
        admission: DurableCodexAdmission,
        identity: &HookIdentity,
        builder: Builder,
    ) -> Result<Arc<CodexRouteEntry>, CodexRouterError>
    where
        Builder: FnOnce(&CodexAdmissionRecord) -> Result<Arc<CodexHookRuntime>, CodexRouterError>,
    {
        let runtime = builder(admission.record())?;
        self.validate_runtime(&runtime, identity)?;
        admission.verify()?;
        Ok(Arc::new(CodexRouteEntry::new(key, runtime, admission)))
    }

    fn validate_scope(&self, identity: &HookIdentity) -> Result<(), CodexRouterError> {
        if identity.terminal_id != self.terminal_id
            || identity.runtime_epoch != self.runtime_epoch
        {
            return Err(CodexRouterError::Scope);
        }
        Ok(())
    }

    fn validate_runtime(
        &self,
        runtime: &CodexHookRuntime,
        identity: &HookIdentity,
    ) -> Result<(), CodexRouterError> {
        if runtime.terminal_id != self.terminal_id || runtime.runtime_epoch != self.runtime_epoch {
            return Err(CodexRouterError::Runtime);
        }
        let lifecycle = runtime
            .lock_lifecycle_anyhow()
            .map_err(|_| CodexRouterError::Runtime)?;
        if lifecycle.owner() != identity {
            return Err(CodexRouterError::Runtime);
        }
        Ok(())
    }

    fn validate_admission_path(
        &self,
        identity: &HookIdentity,
        admission: &DurableCodexAdmission,
    ) -> Result<(), CodexRouterError> {
        let artifacts = crate::codex_hooks::CodexHookArtifacts::new(
            &self.paths,
            self.terminal_id,
            self.runtime_epoch,
            identity.invocation_id,
        )
        .map_err(|_| CodexRouterError::Admission)?;
        let expected =
            crate::codex_hooks::CodexAdmissionStorage::for_identity(&artifacts, identity)
                .map_err(|_| CodexRouterError::Admission)?;
        if expected.path() != admission.path() {
            return Err(CodexRouterError::Admission);
        }
        Ok(())
    }
}

impl BuildReservation {
    fn new(entries: Arc<RouterEntries>, key: CodexRouteKey, gate: Arc<RouteBuildGate>) -> Self {
        Self {
            entries,
            key,
            gate,
            finished: false,
        }
    }

    fn publish(
        mut self,
        entry: Arc<CodexRouteEntry>,
    ) -> Result<Arc<CodexRouteEntry>, CodexRouterError> {
        if let Err(error) = self
            .entries
            .publish_build(self.key, &self.gate, Arc::clone(&entry))
        {
            let outcome = self.entries.fail_build(self.key, &self.gate, error);
            self.finished = true;
            return Err(outcome);
        }
        self.finished = true;
        Ok(entry)
    }

    fn fail(mut self, error: CodexRouterError) -> CodexRouterError {
        let outcome = self.entries.fail_build(self.key, &self.gate, error);
        self.finished = true;
        outcome
    }
}

impl Drop for BuildReservation {
    fn drop(&mut self) {
        if !self.finished {
            self.entries
                .fail_build(self.key, &self.gate, CodexRouterError::Lock);
        }
    }
}

fn verify_replay(
    lease: CodexRouteLease,
    admission: &DurableCodexAdmission,
) -> Result<CodexRouteLease, CodexRouterError> {
    if let Err(error) = admission.verify() {
        return match lease.abort() {
            Ok(()) => Err(error),
            Err(abort_error) => Err(abort_error),
        };
    }
    Ok(lease)
}

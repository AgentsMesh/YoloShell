use super::recovery::CodexRecoveryContext;
use super::router::CodexRecoveredRoute;
use super::runtime::CodexHookRuntime;
use crate::codex_hooks::{
    CodexAdmissionAuthority, CodexRetirementSealArtifact, CodexRetirementState,
    CodexRetirementWriterFence,
};
use anyhow::{Result, ensure};
use std::sync::Arc;

#[must_use]
pub(super) struct CodexRecoveredLiveRoute {
    route: CodexRecoveredRoute,
    writer: CodexRetirementWriterFence,
    admission: CodexAdmissionAuthority,
}

impl CodexRecoveryContext<'_> {
    pub(super) fn recover_live(
        &self,
        writer: CodexRetirementWriterFence,
        admission: CodexAdmissionAuthority,
    ) -> Result<CodexRecoveredLiveRoute> {
        verify_live_state(&writer, &admission)?;
        let route = super::recovery_install::install(
            self.router,
            &admission,
            |record| {
                super::native_first_event::prime(self.paths, &admission)?;
                let request = record.request();
                CodexHookRuntime::with_process_source(
                    self.paths,
                    Arc::clone(&self.registry),
                    self.router.terminal_id(),
                    self.router.runtime_epoch(),
                    self.shell,
                    request.identity.clone(),
                    request.provider_version,
                    Arc::clone(&self.process_source),
                )
                .and_then(|(runtime, _)| {
                    runtime.complete_native_registration(&admission)?;
                    Ok(runtime)
                })
            },
            |_| verify_live_state(&writer, &admission),
        )?;
        #[cfg(test)]
        super::recovery_test_hook::run_after_route_install();
        let recovered = CodexRecoveredLiveRoute {
            route,
            writer,
            admission,
        };
        match recovered.verify() {
            Ok(()) => Ok(recovered),
            Err(error) => Err(recovered.reject(error)),
        }
    }

    pub(super) fn recover_sealed(
        &self,
        writer: CodexRetirementWriterFence,
        admission: CodexAdmissionAuthority,
        seal: CodexRetirementSealArtifact,
    ) -> Result<()> {
        let entry = super::recovery_install::install(
            self.router,
            &admission,
            |_| {
                super::native_first_event::prime_sealed(self.paths, &admission, &writer, &seal)?;
                CodexHookRuntime::recover_sealed_with_process_source(
                    self.paths,
                    Arc::clone(&self.registry),
                    self.router.terminal_id(),
                    self.router.runtime_epoch(),
                    self.shell,
                    &admission,
                    &writer,
                    &seal,
                    Arc::clone(&self.process_source),
                )
                .map(|(runtime, _)| runtime)
            },
            |_| writer.verify_seal(&seal, &admission),
        )?;
        #[cfg(test)]
        super::recovery_test_hook::run_after_route_install();
        if let Err(error) = writer.verify_seal(&seal, &admission) {
            return Err(reject_route(entry, error));
        }
        let readiness = match self.router.begin_retirement(entry.entry()) {
            Ok(readiness) => readiness,
            Err(error) => return Err(reject_route(entry, anyhow::Error::new(error))),
        };
        drop(writer);
        if let Err(error) = super::intake_task::reconcile_recovered_retirement(readiness) {
            return Err(reject_route(entry, error));
        }
        drop(entry.accept());
        Ok(())
    }
}

impl CodexRecoveredLiveRoute {
    pub(super) fn verify(&self) -> Result<()> {
        verify_live_state(&self.writer, &self.admission)
    }

    pub(super) fn accept(self) {
        drop(self.route.accept());
    }

    pub(super) fn rollback(self) -> Result<(), super::router::CodexRouterError> {
        self.route.rollback()
    }

    fn reject(self, cause: anyhow::Error) -> anyhow::Error {
        match self.rollback() {
            Ok(()) => cause,
            Err(error) => anyhow::Error::new(error).context(cause),
        }
    }
}

fn reject_route(route: CodexRecoveredRoute, cause: anyhow::Error) -> anyhow::Error {
    match route.rollback() {
        Ok(()) => cause,
        Err(error) => anyhow::Error::new(error).context(cause),
    }
}

fn verify_live_state(
    writer: &CodexRetirementWriterFence,
    admission: &CodexAdmissionAuthority,
) -> Result<()> {
    ensure!(
        matches!(
            writer.read_state(admission)?,
            CodexRetirementState::Unsealed
        ),
        super::constants::CODEX_ROUTE_RECOVERY_ERROR
    );
    Ok(())
}

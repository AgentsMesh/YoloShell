use super::authority_registry::CodexAuthorityRegistry;
use super::recovery_round::CodexRecoveryRound;
use super::recovery_route::CodexRecoveredLiveRoute;
use super::router::CodexSessionRouter;
use crate::agent::ProcessSource;
use crate::codex_hooks::{
    CodexAdmissionAuthority, CodexHookArtifacts, CodexHookIntake, CodexRetiredState,
    CodexRetirementSeal, CodexRetirementSealArtifact, CodexRetirementState,
    CodexRetirementWriterFence,
};
use crate::process::{ExactProcessGone, ProcessIdentity, ProcessIdentityObservation};
use anyhow::{Context, Result};
use std::sync::Arc;

pub(super) struct CodexRecoveryContext<'a> {
    pub(super) paths: &'a crate::paths::Paths,
    pub(super) registry: Arc<CodexAuthorityRegistry>,
    pub(super) router: &'a CodexSessionRouter,
    pub(super) shell: ProcessIdentity,
    pub(super) process_source: Arc<dyn ProcessSource>,
}

enum RecoveryMode {
    Live,
    Sealed(CodexRetirementSealArtifact),
    Retired(CodexRetiredState),
}

impl<'a> CodexRecoveryContext<'a> {
    pub(super) fn new(
        paths: &'a crate::paths::Paths,
        registry: Arc<CodexAuthorityRegistry>,
        router: &'a CodexSessionRouter,
        shell: ProcessIdentity,
        process_source: Arc<dyn ProcessSource>,
    ) -> Self {
        Self {
            paths,
            registry,
            router,
            shell,
            process_source,
        }
    }

    pub(super) fn recover(&self) -> Result<CodexRecoveryRound> {
        #[cfg(test)]
        super::recovery_test_hook::run();
        self.recover_with(ProcessIdentity::observe, ExactProcessGone::capture)
    }

    #[cfg(test)]
    pub(super) fn recover_observed(
        &self,
        observation: ProcessIdentityObservation,
    ) -> Result<CodexRecoveryRound> {
        self.recover_with(
            move |_| observation,
            move |identity| ExactProcessGone::capture_for_test(identity, observation),
        )
    }

    fn recover_with<Observe, Capture>(
        &self,
        observe: Observe,
        capture: Capture,
    ) -> Result<CodexRecoveryRound>
    where
        Observe: Fn(ProcessIdentity) -> ProcessIdentityObservation,
        Capture: Fn(ProcessIdentity) -> Option<ExactProcessGone>,
    {
        let admissions = crate::codex_hooks::scan_admissions(
            self.paths,
            self.router.terminal_id(),
            self.router.runtime_epoch(),
        )?;
        for admission in &admissions {
            self.router
                .observe_persisted_admission(admission)
                .map_err(anyhow::Error::new)?;
        }
        let mut recovered = Vec::new();
        for admission in admissions {
            match self.recover_admission(admission, &observe, &capture) {
                Ok(Some(route)) => recovered.push(route),
                Ok(None) => {}
                Err(error) => {
                    return Err(CodexRecoveryRound::new(recovered).reject(error));
                }
            }
        }
        let round = CodexRecoveryRound::new(recovered);
        #[cfg(test)]
        super::recovery_test_hook::run_before_round_accept();
        match round.verify() {
            Ok(()) => Ok(round),
            Err(error) => Err(round.reject(error)),
        }
    }

    fn recover_admission<Observe, Capture>(
        &self,
        admission: CodexAdmissionAuthority,
        observe: &Observe,
        capture: &Capture,
    ) -> Result<Option<CodexRecoveredLiveRoute>>
    where
        Observe: Fn(ProcessIdentity) -> ProcessIdentityObservation,
        Capture: Fn(ProcessIdentity) -> Option<ExactProcessGone>,
    {
        let identity = &admission.record().request().identity;
        let artifacts = CodexHookArtifacts::new(
            self.paths,
            self.router.terminal_id(),
            self.router.runtime_epoch(),
            identity.invocation_id,
        )?;
        let writer = admission.acquire_retirement_writer_fence()?;
        let _intake = CodexHookIntake::open_existing_for_identity(&artifacts, identity)?;
        let mode = match writer.read_state(&admission)? {
            CodexRetirementState::Unsealed => {
                self.unsealed_mode(&writer, &admission, observe, capture)?
            }
            CodexRetirementState::Sealed(seal) => RecoveryMode::Sealed(seal),
            CodexRetirementState::Retired(retired) => RecoveryMode::Retired(retired),
        };
        match mode {
            RecoveryMode::Live => self.recover_live(writer, admission).map(Some),
            RecoveryMode::Sealed(seal) => {
                self.recover_sealed(writer, admission, seal).map(|()| None)
            }
            RecoveryMode::Retired(retired) => {
                super::recovery_retired::verify(&artifacts, writer, admission, retired)
                    .map(|()| None)
            }
        }
    }

    fn unsealed_mode<Observe, Capture>(
        &self,
        writer: &CodexRetirementWriterFence,
        admission: &CodexAdmissionAuthority,
        observe: &Observe,
        capture: &Capture,
    ) -> Result<RecoveryMode>
    where
        Observe: Fn(ProcessIdentity) -> ProcessIdentityObservation,
        Capture: Fn(ProcessIdentity) -> Option<ExactProcessGone>,
    {
        let owner = &admission.record().request().identity.process;
        let process = ProcessIdentity {
            pid: owner.pid,
            start_time: owner.start_time,
        };
        match observe(process) {
            ProcessIdentityObservation::Matching => Ok(RecoveryMode::Live),
            ProcessIdentityObservation::Unknown => {
                anyhow::bail!(super::constants::CODEX_RECOVERY_PROCESS_ERROR)
            }
            ProcessIdentityObservation::Gone => {
                let gone =
                    capture(process).context(super::constants::CODEX_RECOVERY_PROCESS_ERROR)?;
                let seal = CodexRetirementSeal::capture(admission, gone)?;
                let publication = writer.publish_seal(&seal, admission)?;
                Ok(RecoveryMode::Sealed(publication.artifact().clone()))
            }
        }
    }
}

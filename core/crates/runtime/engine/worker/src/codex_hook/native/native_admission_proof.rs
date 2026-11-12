use super::{CodexHookPolicy, NativeAdmission, evidence, observe_live};
use crate::agent::ProcessSource;
use crate::agent::hooks::{HookEnvelope, HookIdentity};
use crate::codex_hooks::{CodexAdmissionRecord, CodexNativeRequest};
use crate::process::ProcessIdentity;
use anyhow::{Result, ensure};

impl NativeAdmission {
    pub(in crate::worker::codex_hook) fn envelope(
        &self,
        request: &CodexNativeRequest,
        identity: &HookIdentity,
    ) -> Result<HookEnvelope> {
        let mut envelope = request.envelope(identity)?;
        if envelope.metadata.transcript_path.is_none()
            && let Some(evidence) = &self.evidence
        {
            envelope.metadata.transcript_path = Some(evidence.evidence().path.clone());
        }
        Ok(envelope)
    }

    pub(in crate::worker::codex_hook) fn verify(
        &self,
        source: &dyn ProcessSource,
        shell: ProcessIdentity,
        peer: ProcessIdentity,
        request: &CodexNativeRequest,
        policy: &CodexHookPolicy,
        record: &CodexAdmissionRecord,
    ) -> Result<()> {
        let agent = observe_live(
            source,
            shell,
            ProcessIdentity {
                pid: self.agent.pid,
                start_time: self.agent.start_time,
            },
            peer,
            record,
        )?;
        let (current, proof) = evidence::capture(&agent, request, policy)?;
        let evidence_matches = match (&self.evidence, &proof) {
            // A provisional admission upgrades once the rollout exists.
            (None, _) => true,
            (Some(expected), Some(actual)) => expected.same_evidence(actual),
            (Some(_), None) => false,
        };
        ensure!(
            current.cwd == self.context.cwd
                && current.data_home == self.context.data_home
                && current.real_executable == self.context.real_executable
                && evidence_matches,
            evidence::NATIVE_EVIDENCE
        );
        Ok(())
    }
}

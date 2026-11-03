use anyhow::{Context, Result, ensure};
use uuid::Uuid;
use yolo_agent_authority_contract::AuthorityProcessIdentity;
use yolo_agent_contract::AgentBinding;
use yolo_hooks_runtime::{EventKind, SessionStartSource};
use yolo_paths::identity::FileIdentity;
use yolo_paths::namespace_lease::NamespaceLease;

use super::HookGenerationStore;
use crate::record::{INVALID, increment};
use crate::{GenerationEvidence, GenerationRequest, HookGenerationRecord};

impl HookGenerationStore {
    pub fn apply_in_namespace(
        &self,
        namespace: &NamespaceLease,
        request: &GenerationRequest,
        worker: AuthorityProcessIdentity,
        boot_id: &str,
        evidence: &GenerationEvidence,
        mut commit_fence: impl FnMut() -> Result<()>,
    ) -> Result<HookGenerationRecord> {
        namespace.verify()?;
        request.validate()?;
        ensure!(evidence.verify_own()?, INVALID);
        let loaded = self.load_with_identity_in_namespace(namespace)?;
        let expected = match &loaded {
            Some((record, identity))
                if record.scope == request.scope
                    && record.worker == worker
                    && record.boot_id == boot_id =>
            {
                Some(*identity)
            }
            _ => loaded.as_ref().map(|(_, identity)| *identity),
        };
        let mut record = match &loaded {
            Some((record, _))
                if record.scope == request.scope
                    && record.worker == worker
                    && record.boot_id == boot_id =>
            {
                record.clone()
            }
            _ => {
                let mut record = HookGenerationRecord::first(request, worker, boot_id, evidence)?;
                crate::origin::inherit_session_origin(
                    &mut record,
                    request,
                    loaded.as_ref().map(|(previous, _)| previous),
                );
                record.validate()?;
                self.write_authority(namespace, expected, &record, evidence, || {
                    commit_fence()
                })?;
                return Ok(record);
            }
        };
        ensure!(
            record.provider == request.provider
                && record.provider_version == request.provider_version
                && !record.revoked,
            INVALID
        );
        if request.event_sequence <= record.last_event_sequence {
            let mut reducer = record.reducer()?;
            ensure!(
                reducer.apply(record.envelope(request))?
                    == yolo_hooks_runtime::ApplyDisposition::Replay
                    && record.verify_evidence()?,
                INVALID
            );
            commit_fence()?;
            namespace.verify()?;
            return Ok(record);
        }
        ensure!(
            request.event_sequence == record.last_event_sequence + 1,
            INVALID
        );
        let session_changed = session_changed(&record, request);
        crate::apply::verify_evidence(&record, evidence, session_changed)?;
        let mut reducer = record.reducer()?;
        let disposition = reducer.apply(record.envelope(request))?;
        ensure!(
            matches!(
                disposition,
                yolo_hooks_runtime::ApplyDisposition::Applied
                    | yolo_hooks_runtime::ApplyDisposition::Stale
            ),
            INVALID
        );
        record.capture(&reducer, request, evidence)?;
        record.revision = increment(record.revision)?;
        record.validate()?;
        self.write_authority(namespace, expected, &record, evidence, || {
            commit_fence()
        })?;
        Ok(record)
    }

    pub fn publish_binding_in_namespace(
        &self,
        namespace: &NamespaceLease,
        expected: &HookGenerationRecord,
        mut commit_fence: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        let current = self
            .load_in_namespace(namespace)?
            .context(INVALID)?;
        ensure!(&current == expected && !current.revoked, INVALID);
        let target = current.recovery_binding(true);
        let existing: Option<(AgentBinding, FileIdentity)> =
            if namespace.file_is_missing(&self.binding_path)? {
                None
            } else {
                Some(namespace.read_json_with_identity(&self.binding_path)?)
            };
        if existing
            .as_ref()
            .is_some_and(|(binding, _)| binding == &target)
        {
            ensure!(current.verify_evidence()?, INVALID);
            commit_fence()?;
            namespace.verify()?;
            return Ok(());
        }
        namespace.write_json_expected_with_commit_fence(
            &self.binding_path,
            existing.as_ref().map(|(_, identity)| *identity),
            &target,
            || {
                ensure!(
                    current.verify_evidence()?
                        && self.load_in_namespace(namespace)?.as_ref() == Some(expected),
                    INVALID
                );
                commit_fence()
            },
        )?;
        Ok(())
    }

    pub fn revoke_in_namespace(
        &self,
        namespace: &NamespaceLease,
        terminal_id: Uuid,
        mut commit_fence: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        let Some((mut record, identity)) = self.load_with_identity_in_namespace(namespace)? else {
            return Ok(());
        };
        ensure!(record.scope.terminal_id == terminal_id, INVALID);
        if record.revoked {
            return Ok(());
        }
        let original = record.clone();
        record.revoked = true;
        record.state = yolo_agent_contract::AgentState::Unknown;
        record.binding.state = record.state;
        record.binding.recovery_availability =
            yolo_agent_contract::AgentRecoveryAvailability::Unavailable;
        record.revision = increment(record.revision)?;
        let target = record.clone();
        namespace.write_json_expected_with_commit_fence(
            &self.path,
            Some(identity),
            &target,
            || {
                ensure!(
                    self.load_with_identity_in_namespace(namespace)?
                        .is_some_and(|(current, current_identity)| current == original
                            && current_identity == identity),
                    INVALID
                );
                commit_fence()
            },
        )?;
        Ok(())
    }
}

fn session_changed(record: &HookGenerationRecord, request: &GenerationRequest) -> bool {
    request.provider_session_id != record.provider_session_id
        && matches!(
            request.event,
            EventKind::SessionStart {
                source: SessionStartSource::Clear
                    | SessionStartSource::Resume
                    | SessionStartSource::Fork
            }
        )
}

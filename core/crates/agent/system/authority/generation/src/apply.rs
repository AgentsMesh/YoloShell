use super::GenerationEvidence;
use super::record::{HookGenerationRecord, INVALID};
use anyhow::{Result, ensure};
use yolo_agent_contract::AgentState;
use yolo_hooks_runtime::NormalizedLifecycle;

pub(super) fn activity(state: &NormalizedLifecycle) -> AgentState {
    super::projection::project(state, super::projection::ProjectionRules::GENERATION)
        .unwrap_or(AgentState::Unknown)
}

pub(super) fn verify_evidence(
    record: &HookGenerationRecord,
    evidence: &GenerationEvidence,
    session_changed: bool,
) -> Result<()> {
    ensure!(
        evidence.verify_own()? && evidence.data_home_identity == record.data_home_identity,
        INVALID
    );
    if session_changed {
        return Ok(());
    }
    ensure!(
        evidence.provider_session_id == record.provider_session_id
            && evidence.cwd == std::path::Path::new(&record.binding.cwd)
            && evidence.data_home == std::path::Path::new(&record.binding.data_home)
            && evidence.transcript == std::path::Path::new(&record.binding.evidence_path)
            && evidence.cwd_identity == record.cwd_identity
            && (record.transcript_identity.is_none()
                || evidence.transcript_identity == record.transcript_identity),
        INVALID
    );
    Ok(())
}

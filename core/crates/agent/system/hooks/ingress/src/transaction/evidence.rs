use crate::admission::Admission;
use super::super::constants::INVALID;
use crate::profile::{CanonicalHookEvent, HookCodec};
use super::super::contract::HookRequest;
use anyhow::{Context, Result, ensure};
use std::path::Path;
use yolo_agent_adapter_contract::{AgentAdapter, DetectionContext};
use yolo_agent_authority_contract::AuthorityProcessIdentity;
use yolo_agent_hook_generation::{
    GenerationEvidence, GenerationRequest, GenerationScope, HookMetadata, HookProviderKind,
};
use yolo_hooks_runtime::EventKind;
use yolo_paths::identity::ObjectIdentity;

pub(super) fn generation_request(
    request: &HookRequest,
    codec: &dyn crate::profile::HookCodec,
    event: &crate::profile::CanonicalHookEvent,
) -> GenerationRequest {
    GenerationRequest {
        scope: GenerationScope {
            terminal_id: request.scope.terminal_id,
            runtime_epoch: request.scope.runtime_epoch,
            agent: AuthorityProcessIdentity {
                pid: request.scope.agent.pid,
                start_time: request.scope.agent.start_time,
            },
        },
        event_sequence: request.event_sequence,
        event_id: request.event_id,
        provider: HookProviderKind::Other(codec.profile().provider.into()),
        provider_version: codec.provider_version(),
        provider_session_id: event.provider_session_id.clone(),
        turn_id: event.turn_id.clone(),
        agent_id: event.agent_id.clone(),
        tool_name: event.tool_name.clone(),
        tool_use_id: event.tool_use_id.clone(),
        stop_hook_active: event.stop_hook_active,
        metadata: HookMetadata {
            cwd: Some(event.cwd.clone()),
            transcript_path: Some(event.transcript_path.clone()),
        },
        event: event.event.clone(),
    }
}

pub(super) fn inspect_evidence(
    admitted: &crate::admission::Admission,
    event: &crate::profile::CanonicalHookEvent,
    request: &HookRequest,
    codec: &dyn crate::profile::HookCodec,
    adapter: &dyn AgentAdapter,
) -> Result<GenerationEvidence> {
    let data_home = codec.data_home(&admitted.agent)?;
    let cwd = admitted
        .agent
        .cwd
        .as_ref()
        .context(INVALID)?
        .canonicalize()?;
    ensure!(event.cwd.is_absolute() && event.cwd == cwd, INVALID);
    ensure!(event.transcript_path.is_absolute(), INVALID);
    let path = &canonical_transcript(&event.transcript_path)?;
    ensure!(
        path.starts_with(&data_home)
            && path
                .components()
                .all(|component| matches!(
                    component,
                    std::path::Component::RootDir | std::path::Component::Normal(_)
                )),
        INVALID
    );
    let context = yolo_agent_adapter_contract::DetectionContext::new(
        request.scope.terminal_id,
        cwd.clone(),
        [(codec.kind(), data_home.clone())],
    );
    let evidence = adapter.inspect_evidence_in_context(path, &cwd, &context)?;
    let identity = match evidence.as_ref() {
        Some(evidence) => {
            ensure!(
                evidence.provider_session_id == event.provider_session_id
                    && evidence.cwd == cwd
                    && evidence.path == *path
                    && evidence.data_home == data_home,
                INVALID
            );
            ObjectIdentity::from_file(&std::fs::File::open(path)?)?
        }
        None => {
            ensure!(
                matches!(event.event, EventKind::SessionStart { .. }),
                INVALID
            );
            None
        }
    };
    GenerationEvidence::new(
        codec.kind(),
        codec.executable().to_owned(),
        event.provider_session_id.clone(),
        cwd,
        data_home,
        path.clone(),
        identity,
    )
}

/// Providers build the transcript path from the raw environment value, which
/// may differ from the canonical data home (a symlinked home or macOS `/var`).
/// Canonicalize the existing file, or the nearest existing ancestor plus the
/// missing components, so a session's first event (before the provider creates
/// its transcript directory) and later events record the same evidence path.
pub(super) fn canonical_transcript(path: &Path) -> Result<std::path::PathBuf> {
    if path.exists() {
        return Ok(path.canonicalize()?);
    }
    let mut missing = Vec::new();
    let mut current = path;
    while !current.exists() {
        missing.push(current.file_name().context(INVALID)?.to_owned());
        current = current.parent().context(INVALID)?;
    }
    let mut canonical = current.canonicalize()?;
    for name in missing.into_iter().rev() {
        canonical.push(name);
    }
    Ok(canonical)
}

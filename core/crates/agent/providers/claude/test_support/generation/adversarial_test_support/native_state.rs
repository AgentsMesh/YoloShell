use super::*;
use crate::claude::{AuthoritativeClaudeEvidence, ClaudeAdapter};
use crate::claude_hooks::{
    CLAUDE_NATIVE_SCHEMA_VERSION, ClaudeHookState, ClaudeLifecycleEvent, ClaudeNativeRequest,
    ClaudeSessionEndReason,
};

const EVENT_INCREMENT: u64 = 1;

pub(super) fn persist(
    namespace: &NamespaceLease,
    store: &ClaudeNativeGenerationStore,
    current: &ClaudeNativeGeneration,
    target: AgentState,
) -> Result<()> {
    ensure!(!current.revoked, INVALID);
    let already = match target {
        AgentState::Exited => current.final_session_end,
        _ => !current.final_session_end && current.activity == target,
    };
    let next = if already {
        current.clone()
    } else {
        let event_id = uuid::Uuid::new_v4();
        let provider_session_id = current.binding.provider_session_id.clone();
        let event = match target {
            AgentState::Running | AgentState::WaitingForInput => {
                ClaudeLifecycleEvent::StateChanged {
                    event_id,
                    provider_session_id,
                    state: if target == AgentState::Running {
                        ClaudeHookState::Running
                    } else {
                        ClaudeHookState::WaitingForInput
                    },
                }
            }
            AgentState::Exited | AgentState::Unknown => ClaudeLifecycleEvent::SessionEnd {
                event_id,
                provider_session_id,
                reason: if target == AgentState::Exited {
                    ClaudeSessionEndReason::Other
                } else {
                    ClaudeSessionEndReason::Clear
                },
            },
            AgentState::Active | AgentState::WaitingForPermission => anyhow::bail!(INVALID),
        };
        let request = ClaudeNativeRequest {
            schema_version: CLAUDE_NATIVE_SCHEMA_VERSION,
            scope: current.scope,
            event_sequence: current
                .last_event_sequence
                .checked_add(EVENT_INCREMENT)
                .context(INVALID)?,
            event,
        };
        store.apply_in_namespace(
            namespace,
            &request,
            current.worker,
            &current.boot_id,
            &evidence(current)?,
            || require_gone(current),
        )?
    };
    store.publish_binding_in_namespace(namespace, &next, || require_gone(&next))
}

fn evidence(current: &ClaudeNativeGeneration) -> Result<AuthoritativeClaudeEvidence> {
    let binding = &current.binding;
    let evidence = ClaudeAdapter
        .inspect_authoritative_evidence_with_hook(
            Path::new(&binding.evidence_path),
            Path::new(&binding.cwd),
            Path::new(&binding.data_home),
            || {},
        )?
        .context(INVALID)?;
    ensure!(evidence.identity() == current.transcript_identity, INVALID);
    Ok(evidence)
}

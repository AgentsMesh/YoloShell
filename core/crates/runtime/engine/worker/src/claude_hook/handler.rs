use super::constants::*;
use super::native_environment::ClaudeHookEnvironment;
use crate::claude_hooks::{
    ClaudeInteractionDecision, ClaudeLifecycleEvent, ClaudeNativeRequest, ClaudeNativeResponse,
};
use crate::local_unix::LocalUnixStream;
use crate::worker::WorkerState;
use anyhow::{Context, Result};
use std::sync::Arc;

pub(super) async fn handle(
    mut stream: LocalUnixStream,
    state: Arc<WorkerState>,
    shell: crate::process::ProcessIdentity,
    environment: ClaudeHookEnvironment,
    runtime_active: bool,
) -> Result<()> {
    let peer_pid = super::peer::pid(&stream)?;
    let request = tokio::time::timeout(
        REQUEST_TIMEOUT,
        crate::claude_hooks::read_frame::<_, ClaudeNativeRequest>(&mut stream),
    )
    .await
    .context(REQUEST_TIMEOUT_CONTEXT)??;
    let mut response = rejection(&request);
    let interaction = match &request.event {
        ClaudeLifecycleEvent::InteractionRequested { interaction, .. } => Some(*interaction),
        _ => None,
    };
    if !runtime_active {
        if request.is_valid() && matches!(request.event, ClaudeLifecycleEvent::SessionStart { .. })
        {
            response.code = crate::claude_hooks::CLAUDE_HOOK_NOT_READY.to_owned();
        }
    } else if let Some(interaction) = interaction {
        match super::permission::handle(
            &mut stream,
            state,
            shell,
            &environment,
            peer_pid,
            &request,
        )
        .await
        {
            Ok(decision) => {
                response.accepted = true;
                response.code = crate::claude_hooks::CLAUDE_HOOK_ACCEPTED.to_owned();
                response.interaction = decision.map(|allow| ClaudeInteractionDecision {
                    interaction_id: interaction,
                    allow,
                });
            }
            Err(error) => {
                tracing::warn!(detail = %format!("{error:#}"), "rejected Claude permission request")
            }
        }
    } else {
        match super::native_transaction::execute(&state, shell, &environment, peer_pid, request)
            .await
        {
            Ok(record) => {
                response.accepted = true;
                response.code = crate::claude_hooks::CLAUDE_HOOK_ACCEPTED.to_owned();
                response.invocation_id = Some(record.invocation_id);
                response.revision = Some(record.revision);
            }
            Err(error) => {
                tracing::warn!(detail = %format!("{error:#}"), "rejected Claude native hook request")
            }
        }
    }
    tokio::time::timeout(
        RESPONSE_TIMEOUT,
        crate::claude_hooks::write_frame(&mut stream, &response),
    )
    .await
    .context(RESPONSE_TIMEOUT_CONTEXT)??;
    Ok(())
}

pub(super) fn rejection(request: &ClaudeNativeRequest) -> ClaudeNativeResponse {
    ClaudeNativeResponse {
        schema_version: crate::claude_hooks::CLAUDE_NATIVE_SCHEMA_VERSION,
        accepted: false,
        code: crate::claude_hooks::CLAUDE_HOOK_REJECTED.to_owned(),
        scope: request.scope,
        event_sequence: request.event_sequence,
        event_id: request.event.event_id(),
        invocation_id: None,
        revision: None,
        interaction: None,
    }
}

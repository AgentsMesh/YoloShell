use super::*;
use crate::agent::claude::generation::{
    AuthorityProcessIdentity, AuthorityRuntimeIdentity, ClaudeGenerationAuthority,
    ClaudeGenerationStore, GenerationRecoveryEvent,
};
use crate::claude_hooks::ClaudeHookCredentials;
use crate::claude_hooks::outbox::PendingRecovery;
use anyhow::{Result, ensure};

pub(super) fn recover(
    store: &AgentBindingStore,
    namespace: &crate::namespace_lease::NamespaceLease,
    context: &DetectionContext,
    runtime: Option<AuthorityRuntimeIdentity>,
    authority: Option<&ClaudeGenerationAuthority>,
    previous: Option<&AgentBinding>,
) -> Result<Option<AgentBinding>> {
    let Some(authority) = authority.filter(|value| recoverable(value, runtime)) else {
        return Ok(None);
    };
    let generations = ClaudeGenerationStore::from_binding_path(store.path())?;
    let credentials = credentials(authority);
    let outbox_path = outbox_path(store.path())?;
    if let Some(pending) = crate::claude_hooks::outbox::pending_recovery(&outbox_path, credentials)?
    {
        return recover_outbox(
            store,
            namespace,
            context,
            &generations,
            authority,
            previous,
            &outbox_path,
            credentials,
            pending,
        )
        .map(Some);
    }
    recover_published(store, namespace, context, &generations, authority, previous)
}

#[allow(clippy::too_many_arguments)]
fn recover_outbox(
    store: &AgentBindingStore,
    namespace: &crate::namespace_lease::NamespaceLease,
    context: &DetectionContext,
    generations: &ClaudeGenerationStore,
    authority: &ClaudeGenerationAuthority,
    previous: Option<&AgentBinding>,
    outbox_path: &std::path::Path,
    credentials: ClaudeHookCredentials,
    pending: PendingRecovery,
) -> Result<AgentBinding> {
    ensure!(
        pending.agent.pid == authority.process.pid
            && pending.agent.start_time == authority.process.start_time,
        crate::agent::constants::recovery::OUTBOX_AGENT_MISMATCH
    );
    let events = pending
        .events
        .iter()
        .map(|event| GenerationRecoveryEvent {
            sequence: event.event_sequence,
            event: crate::agent::claude::generation::from_hook_event(&event.event),
            transcript_identity: event.transcript_identity,
        })
        .collect::<Vec<_>>();
    let (candidate, _) = generations.preview_recovery(authority, &events)?;
    let evidence = super::pending_recovery_evidence::inspect(&candidate, &pending)?;
    let (applied, changed) = generations.apply_recovery_in_namespace(
        namespace,
        authority.authentication(),
        authority,
        &events,
    )?;
    ensure!(
        applied == candidate,
        crate::agent::constants::recovery::AUTHORITY_CHANGED
    );
    evidence.verify(&applied)?;
    let binding = if !changed
        && generations.binding_is_published_in_namespace(
            namespace,
            applied.authentication(),
            &applied,
        )?
        && previous.is_some_and(|binding| applied.matches_binding(binding))
    {
        previous
            .cloned()
            .ok_or_else(|| anyhow::anyhow!(crate::agent::constants::recovery::BINDING_MISSING))?
    } else {
        super::pending_recovery_binding::publish(
            store,
            namespace,
            context,
            generations,
            &applied,
            previous,
            &evidence,
        )?
    };
    crate::claude_hooks::outbox::acknowledge_recovery(outbox_path, credentials, &pending)?;
    Ok(binding)
}

fn recover_published(
    store: &AgentBindingStore,
    namespace: &crate::namespace_lease::NamespaceLease,
    context: &DetectionContext,
    generations: &ClaudeGenerationStore,
    authority: &ClaudeGenerationAuthority,
    previous: Option<&AgentBinding>,
) -> Result<Option<AgentBinding>> {
    if authority.transcript_path.is_none() || authority.state == AgentState::Unknown {
        return Ok(None);
    }
    let evidence = super::pending_recovery_evidence::inspect(
        authority,
        &PendingRecovery {
            agent: crate::process::ProcessIdentity {
                pid: authority.process.pid,
                start_time: authority.process.start_time,
            },
            events: Vec::new(),
        },
    )?;
    if generations.binding_is_published_in_namespace(
        namespace,
        authority.authentication(),
        authority,
    )? && previous.is_some_and(|binding| authority.matches_binding(binding))
    {
        return Ok(previous.cloned());
    }
    super::pending_recovery_binding::publish(
        store,
        namespace,
        context,
        generations,
        authority,
        previous,
        &evidence,
    )
    .map(Some)
}

fn outbox_path(binding_path: &std::path::Path) -> Result<std::path::PathBuf> {
    let settings = binding_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!(crate::agent::constants::recovery::BINDING_PARENT_MISSING))?
        .join(crate::claude_settings_artifact::DIRECTORY_NAME);
    crate::claude_hooks::outbox::path(&settings)
}

fn credentials(authority: &ClaudeGenerationAuthority) -> ClaudeHookCredentials {
    ClaudeHookCredentials {
        terminal_id: authority.terminal_id,
        runtime_epoch: authority.runtime_epoch,
        invocation_id: authority.invocation_id,
        hook_token: authority.hook_token,
        helper_capability: authority.helper_capability,
    }
}

fn recoverable(
    authority: &ClaudeGenerationAuthority,
    runtime: Option<AuthorityRuntimeIdentity>,
) -> bool {
    if runtime.is_some_and(|runtime| authority.matches_runtime(runtime)) {
        return false;
    }
    match crate::process::boot_identity_relation(
        &authority.handshake_boot_id,
        &crate::process::boot_id(),
    ) {
        crate::process::BootIdentityRelation::Different => true,
        crate::process::BootIdentityRelation::Unknown => {
            gone(authority.worker) && gone(authority.process)
        }
        crate::process::BootIdentityRelation::Same => {
            gone(authority.worker) && gone(authority.process)
        }
    }
}

fn gone(identity: AuthorityProcessIdentity) -> bool {
    crate::process::process_identity_relation(identity.pid, identity.start_time)
        == crate::process::ProcessIdentityRelation::Gone
}

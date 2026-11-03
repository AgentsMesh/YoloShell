use super::*;
use crate::agent::claude::generation::{
    ClaudeGenerationAuthority, ClaudeGenerationStore, GenerationRecoveryEvent,
};
use crate::claude_hooks::ClaudeHookCredentials;
use crate::claude_hooks::outbox::{PendingRecovery, RecoveryProbe};
use anyhow::{Result, ensure};

const RECOVERY_DEFERRED_LOG: &str =
    "dead parked Claude recovery was isolated from the active binding";

pub(super) fn reconcile(
    store: &AgentBindingStore,
    namespace: &crate::namespace_lease::NamespaceLease,
    generations: &ClaudeGenerationStore,
) -> Result<()> {
    for authority in generations.dead_parked_in_namespace(namespace)? {
        reconcile_authority(store, namespace, generations, authority)?;
    }
    Ok(())
}

pub(super) fn reconcile_isolated(
    store: &AgentBindingStore,
    namespace: &crate::namespace_lease::NamespaceLease,
    generations: &ClaudeGenerationStore,
) -> Result<()> {
    for authority in generations.dead_parked_in_namespace(namespace)? {
        let invocation_id = authority.invocation_id;
        if let Err(error) = reconcile_authority(store, namespace, generations, authority) {
            tracing::warn!(%invocation_id, %error, RECOVERY_DEFERRED_LOG);
        }
    }
    Ok(())
}

fn reconcile_authority(
    store: &AgentBindingStore,
    namespace: &crate::namespace_lease::NamespaceLease,
    generations: &ClaudeGenerationStore,
    authority: ClaudeGenerationAuthority,
) -> Result<()> {
    let outbox = outbox_path(store.path())?;
    let credentials = credentials(&authority);
    match crate::claude_hooks::outbox::probe_recovery(&outbox, credentials)? {
        RecoveryProbe::Blocked => Ok(()),
        RecoveryProbe::Empty => generations.retire_parked_in_namespace(
            namespace,
            authority.authentication(),
            &authority,
        ),
        RecoveryProbe::Pending(pending) => recover(
            namespace,
            generations,
            &outbox,
            authority,
            credentials,
            pending,
        ),
    }
}

fn recover(
    namespace: &crate::namespace_lease::NamespaceLease,
    generations: &ClaudeGenerationStore,
    outbox: &std::path::Path,
    authority: ClaudeGenerationAuthority,
    credentials: ClaudeHookCredentials,
    pending: PendingRecovery,
) -> Result<()> {
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
    let (candidate, _) = generations.preview_recovery(&authority, &events)?;
    let evidence = super::pending_recovery_evidence::inspect(&candidate, &pending)?;
    let (applied, _) = generations.apply_parked_recovery_in_namespace(
        namespace,
        authority.authentication(),
        &authority,
        &events,
    )?;
    ensure!(
        applied == candidate,
        crate::agent::constants::recovery::AUTHORITY_CHANGED
    );
    evidence.verify(&applied)?;
    crate::claude_hooks::outbox::acknowledge_recovery(outbox, credentials, &pending)?;
    generations.retire_parked_in_namespace(namespace, applied.authentication(), &applied)
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

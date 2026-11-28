use super::native_test_support::Fixture;
use crate::claude::generation::native::ClaudeNativeGenerationStore;
use crate::claude_hooks::*;
use crate::{AgentKind, AgentState};
use std::path::Path;

const NEXT_EVENT: u64 = 1;
const RETAINED_HOME: &str = "retained-provider-home";
const PARKED_PROCESS_OFFSET: u32 = 100;

#[test]
fn final_session_end_and_process_absence_publish_confirmed_exit() {
    let fixture = Fixture::new(AgentKind::ClaudeCode);
    let store = ClaudeNativeGenerationStore::from_binding_path(fixture.store.path()).unwrap();
    let record = store
        .load_in_namespace(&fixture.namespace)
        .unwrap()
        .unwrap();
    let evidence = crate::claude::ClaudeAdapter
        .inspect_authoritative_evidence_with_hook(
            Path::new(&record.binding.evidence_path),
            Path::new(&record.binding.cwd),
            Path::new(&record.binding.data_home),
            || {},
        )
        .unwrap()
        .unwrap();
    let request = ClaudeNativeRequest {
        schema_version: CLAUDE_NATIVE_SCHEMA_VERSION,
        scope: record.scope,
        event_sequence: record.last_event_sequence + NEXT_EVENT,
        event: ClaudeLifecycleEvent::SessionEnd {
            event_id: uuid::Uuid::new_v4(),
            provider_session_id: record.binding.provider_session_id.clone(),
            reason: ClaudeSessionEndReason::Other,
        },
    };
    store
        .apply_in_namespace(
            &fixture.namespace,
            &request,
            fixture.runtime.worker,
            &crate::process::boot_id(),
            &evidence,
            || Ok(()),
        )
        .unwrap();
    let exited = fixture.repair().unwrap().unwrap();
    assert_eq!(exited.state, AgentState::Exited);
    assert!(exited.process_pid.is_none() && exited.process_start_time.is_none());
    assert_eq!(fixture.load().unwrap(), Some(exited.clone()));
    assert!(
        fixture
            .store
            .authorize_native_resume_in_namespace(
                &fixture.namespace,
                &fixture.context,
                fixture.runtime,
                &exited
            )
            .unwrap()
    );
}

#[test]
fn replacement_data_home_cannot_reuse_the_original_transcript_inode() {
    for kind in [AgentKind::ClaudeCode, AgentKind::Codex] {
        let fixture = Fixture::new(kind.clone());
        let home = Path::new(&fixture.binding.data_home);
        let retained = home.with_file_name(RETAINED_HOME);
        std::fs::rename(home, &retained).unwrap();
        std::fs::create_dir(home).unwrap();
        let evidence_directory = match kind {
            AgentKind::ClaudeCode => crate::claude::constants::EVIDENCE_DIR,
            _ => crate::codex::constants::EVIDENCE_DIR,
        };
        std::fs::rename(
            retained.join(evidence_directory),
            home.join(evidence_directory),
        )
        .unwrap();
        assert!(fixture.load().is_err());
    }
}

#[test]
fn explicit_clear_revokes_parked_claude_invocations_as_well_as_the_active_one() {
    let fixture = Fixture::new(AgentKind::ClaudeCode);
    let store = ClaudeNativeGenerationStore::from_binding_path(fixture.store.path()).unwrap();
    let first = store
        .load_in_namespace(&fixture.namespace)
        .unwrap()
        .unwrap();
    let original = first.replay_events.front().unwrap().clone();
    let mut second = original.clone();
    second.scope.agent.pid += PARKED_PROCESS_OFFSET;
    second.scope.agent.start_time += u64::from(PARKED_PROCESS_OFFSET);
    if let ClaudeLifecycleEvent::SessionStart { event_id, .. } = &mut second.event {
        *event_id = uuid::Uuid::new_v4();
    }
    let evidence = crate::claude::ClaudeAdapter
        .inspect_authoritative_evidence_with_hook(
            Path::new(&first.binding.evidence_path),
            Path::new(&first.binding.cwd),
            Path::new(&first.binding.data_home),
            || {},
        )
        .unwrap()
        .unwrap();
    store
        .apply_in_namespace(
            &fixture.namespace,
            &second,
            fixture.runtime.worker,
            &crate::process::boot_id(),
            &evidence,
            || Ok(()),
        )
        .unwrap();
    let guard = crate::paths::durable_mutation::SessionDurableMutationGuard::acquire(
        &fixture.paths,
        &fixture.namespace,
        first.scope.terminal_id,
    )
    .unwrap();
    fixture
        .store
        .revoke_native_authorities_with_guard(&guard, || Ok(()))
        .unwrap();
    drop(guard);
    for scope in [original.scope, second.scope] {
        assert!(
            store
                .load_scope_in_namespace(&fixture.namespace, scope)
                .unwrap()
                .unwrap()
                .revoked
        );
    }
    assert!(
        store
            .apply_in_namespace(
                &fixture.namespace,
                &original,
                fixture.runtime.worker,
                &crate::process::boot_id(),
                &evidence,
                || Ok(())
            )
            .is_err()
    );
}

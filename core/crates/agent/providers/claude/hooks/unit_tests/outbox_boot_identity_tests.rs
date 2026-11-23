use super::*;
use crate::claude_hooks::{ClaudeHookState, ClaudeLifecycleEvent};
use uuid::Uuid;

const PREVIOUS_BOOT_ID: &str = "previous-boot";
const COLLIDING_PID: u32 = 75_001;
const COLLIDING_START: u64 = 75_002;
const LEGACY_SCHEMA_VERSION: u64 = 1;
const INVOCATIONS_FIELD: &str = "invocations";
const AGENT_FIELD: &str = stringify!(agent);
const BOOT_FIELD: &str = "boot";
const FORMAT_FIELD: &str = "format";
const UNSUPPORTED_FORMAT: &str = "boot-id-sha256-v0";
const LIVE_OWNER_OUTBOX: &str = "live-owner/state.json";
const FIRST_INVOCATION: usize = 0;

#[test]
fn previous_boot_pid_start_collision_is_authoritatively_gone() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("outbox/state.json");
    let credentials = credentials();
    enqueue_for_recovery(&path, credentials, event(), None, process()).unwrap();
    persistence::update(&path, |state| {
        state.invocations[0].agent =
            ProcessClaim::from_identity_on_boot(process(), PREVIOUS_BOOT_ID);
        Ok(())
    })
    .unwrap();
    let _collision = crate::process::install_test_process_identity_relation(
        COLLIDING_PID,
        COLLIDING_START,
        crate::process::ProcessIdentityRelation::Matching,
    );
    persistence::update(&path, |state| {
        retirement::prune_dead(state);
        Ok(())
    })
    .unwrap();
    let state = persistence::read(&path, |state| Ok(state.clone())).unwrap();
    assert!(state.invocations.is_empty());
    assert_eq!(state.failed_handshakes.len(), 1);
}

#[test]
fn legacy_bootless_schema_fails_closed_without_rewriting_custody() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy/state.json");
    enqueue_for_recovery(&path, credentials(), event(), None, process()).unwrap();
    let current = persistence::read(&path, |state| Ok(state.clone())).unwrap();
    let mut legacy = serde_json::to_value(current).unwrap();
    legacy[crate::agent::constants::SCHEMA_VERSION_FIELD] = LEGACY_SCHEMA_VERSION.into();
    legacy[INVOCATIONS_FIELD][0][AGENT_FIELD]
        .as_object_mut()
        .unwrap()
        .remove(BOOT_FIELD);
    let encoded = serde_json::to_vec(&legacy).unwrap();
    std::fs::write(&path, &encoded).unwrap();

    assert!(persistence::read(&path, |state| Ok(state.clone())).is_err());
    assert_eq!(std::fs::read(path).unwrap(), encoded);
}

#[test]
fn untyped_boot_hash_never_reclaims_a_live_agents_outbox() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(LIVE_OWNER_OUTBOX);
    let live = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
    enqueue_for_recovery(&path, credentials(), event(), None, live).unwrap();
    let original = persistence::read(&path, |state| Ok(state.clone())).unwrap();
    assert_eq!(
        original.invocations[FIRST_INVOCATION].agent.relation(),
        crate::process::ProcessIdentityRelation::Matching
    );
    for missing_format in [true, false] {
        let mut encoded = serde_json::to_value(&original).unwrap();
        let boot = &mut encoded[INVOCATIONS_FIELD][FIRST_INVOCATION][AGENT_FIELD][BOOT_FIELD];
        *boot = serde_json::to_value(crate::process::BootIdentity::from_value(PREVIOUS_BOOT_ID))
            .unwrap();
        if missing_format {
            boot.as_object_mut().unwrap().remove(FORMAT_FIELD);
        } else {
            boot[FORMAT_FIELD] = UNSUPPORTED_FORMAT.into();
        }
        assert!(
            serde_json::from_value::<ProcessClaim>(
                encoded[INVOCATIONS_FIELD][FIRST_INVOCATION][AGENT_FIELD].clone()
            )
            .is_err()
        );
        let bytes = serde_json::to_vec(&encoded).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        let mut mutated = false;
        assert!(
            persistence::update(&path, |state| {
                mutated = true;
                retirement::prune_dead(state);
                Ok(())
            })
            .is_err()
        );
        assert!(!mutated);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(live.matches());
    }
}

fn credentials() -> ClaudeHookCredentials {
    ClaudeHookCredentials {
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        invocation_id: Uuid::new_v4(),
        hook_token: Uuid::new_v4(),
        helper_capability: Uuid::new_v4(),
    }
}

fn event() -> ClaudeLifecycleEvent {
    ClaudeLifecycleEvent::StateChanged {
        event_id: Uuid::new_v4(),
        provider_session_id: Uuid::new_v4().hyphenated().to_string(),
        state: ClaudeHookState::Running,
    }
}

fn process() -> crate::process::ProcessIdentity {
    crate::process::ProcessIdentity {
        pid: COLLIDING_PID,
        start_time: COLLIDING_START,
    }
}

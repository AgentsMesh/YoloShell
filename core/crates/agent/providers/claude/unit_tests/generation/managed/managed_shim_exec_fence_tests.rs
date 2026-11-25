use super::managed_shim_exec_publication_test_support::Case;
use crate::agent::AgentState;

const FOREIGN_BOOT_ID: &str = "foreign-boot";
const UNKNOWN_BOOT_ID: &str = "unknown:managed-shim-test";
const FOREIGN_CWD_COMPONENT: &str = "foreign";
const FOREIGN_TRANSCRIPT_COMPONENT: &str = "foreign-transcript";
const IDENTITY_INCREMENT: u64 = 1;

#[test]
fn admission_is_scoped_to_the_begin_boot_and_open_handshake_deadline() {
    let case = Case::native(AgentState::Running);
    let deadline = case.authority.handshake_deadline_ns;
    assert!(case.accepts_at(
        &case.authority,
        &case.successor,
        &case.predecessor,
        &case.authority.handshake_boot_id,
        deadline - IDENTITY_INCREMENT,
    ));
    for (boot, now) in [
        (FOREIGN_BOOT_ID, deadline - IDENTITY_INCREMENT),
        (UNKNOWN_BOOT_ID, deadline - IDENTITY_INCREMENT),
        (case.authority.handshake_boot_id.as_str(), deadline),
        (
            case.authority.handshake_boot_id.as_str(),
            deadline + IDENTITY_INCREMENT,
        ),
    ] {
        assert!(!case.accepts_at(
            &case.authority,
            &case.successor,
            &case.predecessor,
            boot,
            now,
        ));
    }
}

#[test]
fn admission_only_accepts_the_first_unpublished_session_start() {
    for mutate in authority_mutations() {
        let case = Case::native(AgentState::Running);
        let mut authority = case.authority.clone();
        mutate(&mut authority);
        assert!(!case.accepts_at(
            &authority,
            &case.successor,
            &case.predecessor,
            &authority.handshake_boot_id,
            authority.handshake_deadline_ns - IDENTITY_INCREMENT,
        ));
    }
}

#[test]
fn captured_admission_rejects_any_replacement_of_the_exact_predecessor() {
    let case = Case::native(AgentState::Running);
    let admission = case.admission(&case.authority, true).unwrap();
    assert!(admission.accepts(Some(&case.predecessor)).unwrap());

    let mut replacement = case.predecessor.clone();
    replacement.updated_at_ms += IDENTITY_INCREMENT as i64;
    assert!(!admission.accepts(Some(&replacement)).unwrap());
    assert!(!admission.accepts(None).unwrap());
}

#[test]
fn ordinary_hook_publication_cannot_consume_the_special_predecessor() {
    let case = Case::native(AgentState::Running);
    let admission = case.admission(&case.authority, false).unwrap();
    assert!(!admission.accepts(Some(&case.predecessor)).unwrap());
}

#[test]
fn every_first_session_replay_fact_is_part_of_the_admission() {
    for mutate in replay_mutations() {
        let case = Case::native(AgentState::Running);
        let mut authority = case.authority.clone();
        mutate(authority.replay_events.front_mut().unwrap());
        assert!(!case.accepts_at(
            &authority,
            &case.successor,
            &case.predecessor,
            &authority.handshake_boot_id,
            authority.handshake_deadline_ns - IDENTITY_INCREMENT,
        ));
    }
}

type AuthorityMutation = fn(&mut super::super::ClaudeGenerationAuthority);

fn authority_mutations() -> Vec<AuthorityMutation> {
    vec![
        |value| value.binding_revision = Some(value.revision),
        |value| value.discover_session = true,
        |value| value.revision += IDENTITY_INCREMENT,
        |value| value.generation += IDENTITY_INCREMENT,
        |value| value.last_event_sequence += IDENTITY_INCREMENT,
        |value| value.replay_events.clear(),
        |value| value.provider_session_id = uuid::Uuid::new_v4().hyphenated().to_string(),
        |value| value.cwd.push(FOREIGN_CWD_COMPONENT),
        |value| value.state = AgentState::Exited,
        |value| value.mode = super::super::InvocationMode::Resume,
        |value| {
            let event = value.replay_events.front().unwrap().clone();
            value.replay_events.push_back(event);
        },
    ]
}

type ReplayMutation = fn(&mut super::super::types::AppliedGenerationEvent);

fn replay_mutations() -> Vec<ReplayMutation> {
    vec![
        |value| value.sequence += IDENTITY_INCREMENT,
        |value| value.generation += IDENTITY_INCREMENT,
        |value| {
            let super::super::GenerationEvent::SessionStart {
                event_id,
                provider_session_id,
                state,
                ..
            } = &value.event
            else {
                unreachable!();
            };
            value.event = super::super::GenerationEvent::StateChanged {
                event_id: *event_id,
                provider_session_id: provider_session_id.clone(),
                state: *state,
            };
        },
        |value| {
            let super::super::GenerationEvent::SessionStart { source, .. } = &mut value.event
            else {
                unreachable!();
            };
            *source = super::super::SessionStartSource::Resume;
        },
        |value| {
            let super::super::GenerationEvent::SessionStart { cwd, .. } = &mut value.event else {
                unreachable!();
            };
            cwd.push(FOREIGN_CWD_COMPONENT);
        },
        |value| {
            let super::super::GenerationEvent::SessionStart {
                transcript_path, ..
            } = &mut value.event
            else {
                unreachable!();
            };
            transcript_path.push(FOREIGN_TRANSCRIPT_COMPONENT);
        },
        |value| {
            let super::super::GenerationEvent::SessionStart { state, .. } = &mut value.event else {
                unreachable!();
            };
            *state = AgentState::WaitingForInput;
        },
    ]
}

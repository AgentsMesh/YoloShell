use super::fixture::Fixture;
use crate::claude_hooks::{ClaudeLifecycleEvent, ClaudeSessionStartSource};

const NEXT_PROCESS: u64 = 1;
const CLOCK_TICK: std::time::Duration = std::time::Duration::from_millis(2);
#[cfg(target_os = "macos")]
const PREVIOUS_BOOT: &str = "macos:550e8400-e29b-41d4-a716-446655440000";
#[cfg(not(target_os = "macos"))]
const PREVIOUS_BOOT: &str = "550e8400-e29b-41d4-a716-446655440000";

#[test]
fn new_native_resume_invocation_preserves_the_provider_sessions_first_detection() {
    let fixture = Fixture::new();
    let first = fixture.apply(&fixture.start).unwrap();
    std::thread::sleep(CLOCK_TICK);
    let mut resume = fixture.start.clone();
    resume.scope.runtime_epoch = uuid::Uuid::new_v4();
    resume.scope.agent.start_time += NEXT_PROCESS;
    let ClaudeLifecycleEvent::SessionStart {
        event_id, source, ..
    } = &mut resume.event
    else {
        unreachable!();
    };
    *event_id = uuid::Uuid::new_v4();
    *source = ClaudeSessionStartSource::Resume;
    let next = fixture.apply(&resume).unwrap();
    assert_ne!(next.scope, first.scope);
    assert_ne!(next.invocation_id, first.invocation_id);
    assert_eq!(next.binding.detected_at_ms, first.binding.detected_at_ms);
    assert!(next.binding.updated_at_ms > first.binding.updated_at_ms);
    assert_eq!(fixture.apply(&resume).unwrap(), next);
}

#[test]
fn revoked_and_fresh_invocations_cannot_inherit_the_previous_origin() {
    for revoked in [false, true] {
        let fixture = Fixture::new();
        let first = fixture.apply(&fixture.start).unwrap();
        if revoked {
            fixture
                .store
                .revoke_in_namespace(&fixture.namespace, &first, || Ok(()))
                .unwrap();
        }
        std::thread::sleep(CLOCK_TICK);
        let mut next = fixture.start.clone();
        next.scope.runtime_epoch = uuid::Uuid::new_v4();
        next.scope.agent.start_time += NEXT_PROCESS;
        let ClaudeLifecycleEvent::SessionStart {
            event_id, source, ..
        } = &mut next.event
        else {
            unreachable!();
        };
        *event_id = uuid::Uuid::new_v4();
        if revoked {
            *source = ClaudeSessionStartSource::Resume;
        }
        let next = fixture.apply(&next).unwrap();
        assert!(next.binding.detected_at_ms > first.binding.detected_at_ms);
    }
}

#[test]
fn changed_evidence_objects_cannot_supply_a_session_origin() {
    let fixture = Fixture::new();
    let first = fixture.apply(&fixture.start).unwrap();
    let other = Fixture::new();
    let foreign = other.apply(&other.start).unwrap();
    let mut request = fixture.start.clone();
    let ClaudeLifecycleEvent::SessionStart { source, .. } = &mut request.event else {
        unreachable!();
    };
    *source = ClaudeSessionStartSource::Resume;
    let mut next = first.clone();
    next.binding.detected_at_ms += NEXT_PROCESS as i64;
    next.transcript_identity = foreign.transcript_identity;
    let expected = next.binding.detected_at_ms;
    super::session_origin::inherit(&mut next, &request, &[first]);
    assert_eq!(next.binding.detected_at_ms, expected);
}

#[test]
fn in_process_resume_and_later_crash_resume_share_the_provider_origin() {
    let fixture = Fixture::new();
    let first = fixture.apply(&fixture.start).unwrap();
    std::thread::sleep(CLOCK_TICK);
    let (mut other, evidence) = fixture.other_session();
    other.scope.agent.start_time += NEXT_PROCESS;
    let switched = fixture
        .store
        .apply_in_namespace(
            &fixture.namespace,
            &other,
            fixture.worker,
            &fixture.boot_id,
            &evidence,
            || Ok(()),
        )
        .unwrap();
    assert!(switched.binding.detected_at_ms > first.binding.detected_at_ms);
    let mut resume = fixture.start.clone();
    resume.scope = other.scope;
    resume.event_sequence += NEXT_PROCESS;
    let ClaudeLifecycleEvent::SessionStart {
        event_id, source, ..
    } = &mut resume.event
    else {
        unreachable!();
    };
    *event_id = uuid::Uuid::new_v4();
    *source = ClaudeSessionStartSource::Resume;
    let in_process = fixture.apply(&resume).unwrap();
    assert_eq!(in_process.invocation_id, switched.invocation_id);
    assert_eq!(
        in_process.binding.detected_at_ms,
        first.binding.detected_at_ms
    );
    assert_eq!(fixture.apply(&resume).unwrap(), in_process);

    resume.scope.runtime_epoch = uuid::Uuid::new_v4();
    resume.scope.agent.start_time += NEXT_PROCESS;
    resume.event_sequence = crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE;
    let after_crash = fixture.apply(&resume).unwrap();
    assert_ne!(after_crash.invocation_id, in_process.invocation_id);
    assert_eq!(
        after_crash.binding.detected_at_ms,
        in_process.binding.detected_at_ms
    );
}

#[test]
fn clear_into_a_different_session_starts_a_new_detection_origin() {
    let fixture = Fixture::new();
    let first = fixture.apply(&fixture.start).unwrap();
    std::thread::sleep(CLOCK_TICK);
    let (mut clear, evidence) = fixture.other_session();
    clear.event_sequence += NEXT_PROCESS;
    let ClaudeLifecycleEvent::SessionStart { source, .. } = &mut clear.event else {
        unreachable!();
    };
    *source = ClaudeSessionStartSource::Clear;
    let next = fixture
        .store
        .apply_in_namespace(
            &fixture.namespace,
            &clear,
            fixture.worker,
            &fixture.boot_id,
            &evidence,
            || Ok(()),
        )
        .unwrap();
    assert_eq!(next.invocation_id, first.invocation_id);
    assert!(next.binding.detected_at_ms > first.binding.detected_at_ms);
}

#[test]
fn previous_boot_fixture_checks_the_installed_record_without_requiring_old_bytes() {
    let fixture = Fixture::new();
    let first = fixture.apply(&fixture.start).unwrap();
    let binding = fixture.paths.agent_binding(first.scope.terminal_id);
    crate::claude::generation_test_hook::persist_native_previous_boot(
        &binding,
        first.scope.runtime_epoch,
        &first.boot_id,
        PREVIOUS_BOOT,
    )
    .unwrap();
    let mut expected = first;
    expected.boot_id = PREVIOUS_BOOT.into();
    assert_eq!(
        fixture.store.load_in_namespace(&fixture.namespace).unwrap(),
        Some(expected)
    );
}

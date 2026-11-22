use super::*;

const NEXT_SEQUENCE: u64 = 1;
const LATER_TRANSCRIPT: &str = "later.jsonl";

fn replay(
    fixture: &Fixture,
    credentials: ClaudeHookCredentials,
    sequence: u64,
    event_id: uuid::Uuid,
    agent: crate::process::ProcessIdentity,
) -> bool {
    fence_session_start_replay(&fixture.path, credentials, sequence, event_id, agent, Ok).unwrap()
}

#[test]
fn prepared_receipts_do_not_authorize_missing_artifacts() {
    let fixture = Fixture::new();
    let check = || {
        replay(
            &fixture,
            fixture.credentials,
            fixture.event_sequence,
            fixture.event_id,
            fixture.agent,
        )
    };
    assert!(!check());
    fixture.prepare(before(), after()).unwrap();
    assert!(!check());
    fixture.commit(after()).unwrap();
    assert!(check());
}

#[test]
fn receipt_replay_is_bound_to_all_credentials_and_agent_identity() {
    let fixture = Fixture::new();
    fixture.prepare(before(), after()).unwrap();
    fixture.commit(after()).unwrap();
    let check = |credentials, agent| {
        replay(
            &fixture,
            credentials,
            fixture.event_sequence,
            fixture.event_id,
            agent,
        )
    };
    for mutate in [
        |value: &mut ClaudeHookCredentials| value.terminal_id = uuid::Uuid::new_v4(),
        |value: &mut ClaudeHookCredentials| value.runtime_epoch = uuid::Uuid::new_v4(),
        |value: &mut ClaudeHookCredentials| value.invocation_id = uuid::Uuid::new_v4(),
        |value: &mut ClaudeHookCredentials| value.hook_token = uuid::Uuid::new_v4(),
        |value: &mut ClaudeHookCredentials| value.helper_capability = uuid::Uuid::new_v4(),
    ] {
        let mut credentials = fixture.credentials;
        mutate(&mut credentials);
        assert!(!check(credentials, fixture.agent));
    }
    let mut agent = fixture.agent;
    agent.pid += 1;
    assert!(!check(fixture.credentials, agent));
    agent = fixture.agent;
    agent.start_time += 1;
    assert!(!check(fixture.credentials, agent));
}

#[test]
fn an_established_handshake_requires_the_exact_pending_later_event() {
    let fixture = Fixture::new();
    fixture.prepare(before(), after()).unwrap();
    fixture.commit(after()).unwrap();
    let check = |sequence, event_id| {
        replay(
            &fixture,
            fixture.credentials,
            sequence,
            event_id,
            fixture.agent,
        )
    };
    assert!(!check(fixture.event_sequence, uuid::Uuid::new_v4()));
    assert!(!check(
        fixture.event_sequence - NEXT_SEQUENCE,
        fixture.event_id
    ));
    let later = fixture.event_sequence + NEXT_SEQUENCE;
    let event = session_start(fixture._directory.path().join(LATER_TRANSCRIPT));
    let ClaudeLifecycleEvent::SessionStart { event_id, .. } = event else {
        unreachable!()
    };
    assert!(!check(later, event_id));
    let transcript = fixture._directory.path().join(LATER_TRANSCRIPT);
    std::fs::write(&transcript, b"later").unwrap();
    let identity = crate::paths::identity::ObjectIdentity::from_file(
        &std::fs::File::open(transcript).unwrap(),
    )
    .unwrap()
    .unwrap();
    enqueue_for_recovery(
        &fixture.path,
        fixture.credentials,
        event,
        Some(identity),
        fixture.agent,
    )
    .unwrap();
    assert!(check(later, event_id));
    assert!(!check(later, uuid::Uuid::new_v4()));
}

use super::test_support::{CODEX_VERSION, FIRST_POST_START_SEQUENCE, envelope, identity, started};
use super::{EventKind, MAX_REPLAY_WINDOW, HookProviderKind, Reducer};

#[test]
fn replay_window_remains_bounded_after_many_events() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut reducer, &expected);
    let first = FIRST_POST_START_SEQUENCE;
    let last = first + MAX_REPLAY_WINDOW as u64 + super::FIRST_SEQUENCE;
    for sequence in first..=last {
        reducer
            .apply(envelope(
                &expected,
                CODEX_VERSION,
                sequence,
                EventKind::UserPromptSubmit,
            ))
            .unwrap();
    }
    assert_eq!(reducer.state().history.len(), MAX_REPLAY_WINDOW);
    assert_eq!(reducer.state().event_ids.len(), MAX_REPLAY_WINDOW);
}

#[test]
fn late_hook_after_process_exit_is_rejected_without_mutation() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut reducer, &expected);
    assert_eq!(
        reducer.process_exit(),
        super::ProcessExitDisposition::Unconfirmed
    );
    let event = envelope(
        &expected,
        CODEX_VERSION,
        FIRST_POST_START_SEQUENCE,
        EventKind::UserPromptSubmit,
    );
    assert_eq!(
        reducer.apply(event),
        Err(super::HookViolation::ProcessAlreadyExited)
    );
    assert_eq!(reducer.state().process, super::ProcessState::Exited);
}

#[test]
fn exact_replay_after_process_exit_remains_idempotent() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    let event = envelope(
        &expected,
        CODEX_VERSION,
        super::FIRST_SEQUENCE,
        EventKind::SessionStart {
            source: super::SessionStartSource::Startup,
        },
    );
    assert_eq!(
        reducer.apply(event.clone()),
        Ok(super::ApplyDisposition::Applied)
    );
    reducer.process_exit();
    assert_eq!(reducer.apply(event), Ok(super::ApplyDisposition::Replay));
}

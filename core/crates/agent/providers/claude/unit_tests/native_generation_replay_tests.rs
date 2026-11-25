use super::*;

const SECOND_RECORD: usize = 1;
const PID_STEP: u32 = 1;

#[test]
fn multiple_invocations_require_unique_ids_scopes_and_the_same_yolo_session() {
    let fixture = Fixture::new();
    let mut original = document(&fixture);
    let mut request = fixture.start.clone();
    request.scope.agent.pid += PID_STEP;
    let second = reducer::first(
        &request,
        fixture.worker,
        &fixture.boot_id,
        &fixture.evidence,
    )
    .unwrap();
    original.invocations.push(second);
    original.validate().unwrap();
    let mutations: &[fn(&mut NativeGenerations)] = &[
        |value| value.invocations[SECOND_RECORD].invocation_id = Uuid::nil(),
        |value| {
            value.invocations[SECOND_RECORD].invocation_id =
                value.invocations[FIRST_RECORD].invocation_id
        },
        |value| value.invocations[SECOND_RECORD].scope = value.invocations[FIRST_RECORD].scope,
        |value| value.invocations[SECOND_RECORD].scope.terminal_id = Uuid::new_v4(),
    ];
    for mutate in mutations {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert_eq!(changed.validate().unwrap_err().to_string(), INVALID);
    }
}

#[test]
fn retained_native_replay_requires_valid_events_contiguous_sequences_and_unique_ids() {
    let fixture = Fixture::new();
    let mut original = document(&fixture);
    let record = &mut original.invocations[FIRST_RECORD];
    let event = fixture.state_event(record.last_event_sequence + INCREMENT);
    reducer::apply(record, &event, &fixture.evidence).unwrap();
    original.validate().unwrap();
    let mutations: &[fn(&mut ClaudeNativeGeneration)] = &[
        |value| value.replay_events[FIRST_RECORD].schema_version += SCHEMA_STEP,
        |value| value.replay_events[FIRST_RECORD].scope.runtime_epoch = Uuid::new_v4(),
        |value| value.replay_events[SECOND_RECORD].event_sequence += INCREMENT,
        |value| {
            let prior = value.replay_events[FIRST_RECORD].event.event_id();
            let crate::claude_hooks::ClaudeLifecycleEvent::StateChanged { event_id, .. } =
                &mut value.replay_events[SECOND_RECORD].event
            else {
                unreachable!()
            };
            *event_id = prior;
        },
        |value| value.last_event_sequence += INCREMENT,
    ];
    for mutate in mutations {
        let mut changed = original.clone();
        mutate(&mut changed.invocations[FIRST_RECORD]);
        assert_eq!(changed.validate().unwrap_err().to_string(), INVALID);
    }
}

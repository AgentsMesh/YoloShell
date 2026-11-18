use crate::state::{CompactionPhase, NormalizedLifecycle};
use crate::test_support::{CLAUDE_VERSION, SESSION, claude_envelope, claude_started, identity};
use crate::{
    AttentionState, ChildLifecycle, EventSequence, HookProviderKind, Reducer, SnapshotViolation,
    StopCandidate, TurnState,
};

pub const CHILD: &str = "validation-child";
pub const TOOL: &str = "validation-tool";
pub const CWD: &str = "/validation-project";
pub const TRANSCRIPT: &str = "/validation-project/transcript.jsonl";
pub type Mutation = (&'static str, fn(&mut NormalizedLifecycle));

pub fn initial() -> NormalizedLifecycle {
    NormalizedLifecycle::new(identity(), HookProviderKind::Claude, CLAUDE_VERSION)
}

pub fn degraded() -> NormalizedLifecycle {
    let mut reducer = reducer();
    let state = reducer.state();
    let sequence = state.last_sequence.next().unwrap();
    let mut event = claude_envelope(
        &state.expected_identity,
        sequence.0,
        crate::EventKind::SubagentStop,
    );
    event.agent_id = Some(CHILD.into());
    reducer.apply(event).unwrap();
    reducer.snapshot().unwrap();
    reducer.state().clone()
}

pub fn terminal() -> NormalizedLifecycle {
    let mut reducer = reducer();
    let state = reducer.state();
    let sequence = state.last_sequence.next().unwrap();
    let event = claude_envelope(
        &state.expected_identity,
        sequence.0,
        crate::EventKind::SessionEnd {
            reason: crate::SessionEndReason::Other,
        },
    );
    reducer.apply(event).unwrap();
    reducer.snapshot().unwrap();
    reducer.state().clone()
}

pub fn exited() -> NormalizedLifecycle {
    let mut reducer = reducer();
    let state = reducer.state();
    let sequence = state.last_sequence.next().unwrap();
    let mut event = claude_envelope(
        &state.expected_identity,
        sequence.0,
        crate::EventKind::SubagentStart,
    );
    event.agent_id = Some(CHILD.into());
    reducer.apply(event).unwrap();
    reducer.process_exit();
    reducer.snapshot().unwrap();
    reducer.state().clone()
}

fn reducer() -> Reducer {
    let identity = identity();
    let mut reducer = Reducer::new(identity.clone(), HookProviderKind::Claude, CLAUDE_VERSION).unwrap();
    claude_started(&mut reducer, &identity);
    reducer
}

pub fn stop() -> StopCandidate {
    StopCandidate {
        event_id: uuid::Uuid::new_v4(),
        turn_id: Some(SESSION.into()),
        continued: false,
    }
}

pub fn phase() -> CompactionPhase {
    CompactionPhase::begin(TurnState::Running, EventSequence::FIRST)
}

pub fn inactive_child() -> ChildLifecycle {
    ChildLifecycle {
        active: false,
        turn: TurnState::Unknown,
        attention: AttentionState::None,
        active_tools: Default::default(),
        stop_candidate: None,
    }
}

pub fn rejects_each(
    original: &NormalizedLifecycle,
    validate: impl Fn(&NormalizedLifecycle) -> Result<(), SnapshotViolation>,
    mutations: &[Mutation],
) {
    crate::snapshot::validation::validate_lifecycle(original).unwrap();
    validate(original).unwrap();
    for (field, mutate) in mutations {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert_ne!(&changed, original, "mutation did not change {field}");
        assert_eq!(
            validate(&changed),
            Err(SnapshotViolation::InvalidState),
            "{field}"
        );
    }
    validate(original).unwrap();
}

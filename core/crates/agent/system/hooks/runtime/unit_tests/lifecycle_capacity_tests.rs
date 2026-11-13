use super::test_support::{CODEX_VERSION, FIRST_POST_START_SEQUENCE, envelope, identity, started};
use super::{
    ApplyDisposition, EventKind, HookEnvelope, InstrumentationState,
    MAX_RETAINED_LIFECYCLE_ENTRIES, HookProviderKind, Reducer, SessionStartSource, TurnState,
};

const TOOL_PREFIX: &str = "capacity-tool";
const CHILD_PREFIX: &str = "capacity-child";
const OVERFLOW_SUFFIX: &str = "overflow";
const OVERFLOW_CWD: &str = "/tmp/yolo-capacity-cwd";
const OVERFLOW_TRANSCRIPT: &str = "/tmp/yolo-capacity-transcript";
const ROLLING_WINDOW_MULTIPLIER: usize = 3;

struct Scenario {
    identity: super::HookIdentity,
    reducer: Reducer,
    sequence: u64,
}

impl Scenario {
    fn new() -> Self {
        let identity = identity();
        let mut reducer =
            Reducer::new(identity.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
        started(&mut reducer, &identity);
        Self {
            identity,
            reducer,
            sequence: FIRST_POST_START_SEQUENCE,
        }
    }

    fn event(&self, event: EventKind) -> HookEnvelope {
        envelope(&self.identity, CODEX_VERSION, self.sequence, event)
    }

    fn apply(&mut self, event: HookEnvelope) -> ApplyDisposition {
        let disposition = self.reducer.apply(event).unwrap();
        self.sequence += super::FIRST_SEQUENCE;
        disposition
    }

    fn tool(&self, name: String, event: EventKind) -> HookEnvelope {
        let mut envelope = self.event(event);
        envelope.tool_use_id = Some(name);
        envelope
    }

    fn child(&self, name: String, event: EventKind) -> HookEnvelope {
        let mut envelope = self.event(event);
        envelope.agent_id = Some(name);
        envelope
    }
}

#[test]
fn aggregate_tool_capacity_degrades_durably_and_recovers_at_a_session_boundary() {
    let mut scenario = Scenario::new();
    for index in 0..MAX_RETAINED_LIFECYCLE_ENTRIES {
        let event = scenario.tool(format!("{TOOL_PREFIX}-{index}"), EventKind::PreToolUse);
        assert_eq!(scenario.apply(event), ApplyDisposition::Applied);
    }
    let snapshot = scenario.reducer.snapshot().unwrap();
    let encoded = serde_json::to_vec(&snapshot).unwrap();
    assert!(encoded.len() < yolo_storage::DEFAULT_JSON_MAX_BYTES);
    let restored = Reducer::restore(
        snapshot,
        scenario.identity.clone(),
        HookProviderKind::Codex,
        CODEX_VERSION,
    )
    .unwrap();
    assert_eq!(restored.state(), scenario.reducer.state());

    let mut overflow = scenario.tool(
        format!("{TOOL_PREFIX}-{OVERFLOW_SUFFIX}"),
        EventKind::PreToolUse,
    );
    overflow.metadata.cwd = Some(OVERFLOW_CWD.into());
    overflow.metadata.transcript_path = Some(OVERFLOW_TRANSCRIPT.into());
    assert_eq!(scenario.apply(overflow), ApplyDisposition::Applied);
    assert_eq!(
        scenario.reducer.state().instrumentation,
        InstrumentationState::Unavailable
    );
    assert!(scenario.reducer.state().active_tools.is_empty());
    let degraded_snapshot = scenario.reducer.snapshot().unwrap();
    let degraded = Reducer::restore(
        degraded_snapshot,
        scenario.identity.clone(),
        HookProviderKind::Codex,
        CODEX_VERSION,
    )
    .unwrap();
    assert_eq!(degraded.state(), scenario.reducer.state());

    let ignored = scenario.tool(format!("{TOOL_PREFIX}-0"), EventKind::PostToolUse);
    assert_eq!(scenario.apply(ignored), ApplyDisposition::Stale);
    let clear = scenario.event(EventKind::SessionStart {
        source: SessionStartSource::Clear,
    });
    assert_eq!(scenario.apply(clear), ApplyDisposition::Applied);
    assert_eq!(
        scenario.reducer.state().instrumentation,
        InstrumentationState::Authoritative
    );
    assert_eq!(scenario.reducer.state().turn, TurnState::Idle);
}

#[test]
fn child_and_child_tool_entries_share_one_budget() {
    let mut scenario = Scenario::new();
    let child_name = format!("{CHILD_PREFIX}-active");
    let start = scenario.child(child_name.clone(), EventKind::SubagentStart);
    scenario.apply(start);
    for index in 1..MAX_RETAINED_LIFECYCLE_ENTRIES {
        let mut tool = scenario.tool(format!("{TOOL_PREFIX}-{index}"), EventKind::PreToolUse);
        tool.agent_id = Some(child_name.clone());
        scenario.apply(tool);
    }
    let overflow = scenario.tool(
        format!("{TOOL_PREFIX}-{OVERFLOW_SUFFIX}"),
        EventKind::PreToolUse,
    );
    assert_eq!(scenario.apply(overflow), ApplyDisposition::Applied);
    assert_eq!(
        scenario.reducer.state().instrumentation,
        InstrumentationState::Unavailable
    );
}

#[test]
fn rolling_stopped_children_are_collected_beyond_the_history_window() {
    let mut scenario = Scenario::new();
    let cycles = MAX_RETAINED_LIFECYCLE_ENTRIES * ROLLING_WINDOW_MULTIPLIER;
    for index in 0..cycles {
        let name = format!("{CHILD_PREFIX}-{index}");
        let start = scenario.child(name.clone(), EventKind::SubagentStart);
        scenario.apply(start);
        let stop = scenario.child(name, EventKind::SubagentStop);
        scenario.apply(stop);
    }
    assert!(scenario.reducer.state().children.len() < MAX_RETAINED_LIFECYCLE_ENTRIES);
    let snapshot = scenario.reducer.snapshot().unwrap();
    let encoded = serde_json::to_vec(&snapshot).unwrap();
    assert!(encoded.len() < yolo_storage::DEFAULT_JSON_MAX_BYTES);
    let restored = Reducer::restore(
        snapshot,
        scenario.identity.clone(),
        HookProviderKind::Codex,
        CODEX_VERSION,
    )
    .unwrap();
    assert_eq!(restored.state(), scenario.reducer.state());

    let mut late = scenario.child(format!("{CHILD_PREFIX}-0"), EventKind::PermissionRequest);
    late.turn_id = Some(OVERFLOW_SUFFIX.to_owned());
    late.metadata.cwd = Some(OVERFLOW_CWD.into());
    late.metadata.transcript_path = Some(OVERFLOW_TRANSCRIPT.into());
    assert_eq!(scenario.apply(late), ApplyDisposition::Applied);
    assert_eq!(
        scenario.reducer.state().instrumentation,
        InstrumentationState::Unavailable
    );
    let degraded = scenario.reducer.snapshot().unwrap();
    let restored = Reducer::restore(
        degraded,
        scenario.identity,
        HookProviderKind::Codex,
        CODEX_VERSION,
    )
    .unwrap();
    assert_eq!(restored.state(), scenario.reducer.state());
}

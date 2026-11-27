use super::claude_scenario::*;
use super::claude_transport::{JSON_LINE_ENDING, JsonLineHookAdapter};
use super::claude_wire::*;
use std::path::PathBuf;

const YOLO_SESSION_ID: &str = "11111111-1111-4111-8111-111111111111";
const RUNTIME_EPOCH: &str = "22222222-2222-4222-8222-222222222222";
const INVOCATION_ID: &str = "33333333-3333-4333-8333-333333333333";
const HOOK_TOKEN: &str = "44444444-4444-4444-8444-444444444444";
const EVENT_ID: &str = "5aaaaaaa-5555-4555-8555-555555555555";
const LATE_EVENT_ID: &str = "7aaaaaaa-7777-4777-8777-777777777777";
const AGENT_SESSION_ID: &str = "66666666-6666-4666-8666-666666666666";
const TEST_CWD: &str = "/tmp/yolo-scenario/project";
const TEST_TRANSCRIPT: &str =
    "/tmp/yolo-scenario/claude/projects/-project/66666666-6666-4666-8666-666666666666.jsonl";
const WORKER_PID: u32 = 101;
const SHELL_PID: u32 = 102;
const AGENT_PID: u32 = 103;
const HOOK_PID: u32 = 104;
const START_TIME_BASE: u64 = 200;
const OUT_OF_ORDER_FRAME_COUNT: usize = 3;
const JSON_LINE_FRAME_COUNT: usize = 2;

#[test]
fn enabled_scenario_preserves_duplicate_and_out_of_order_frames() {
    let first = start_frame();
    let mut late = first.clone();
    late.event_id = LATE_EVENT_ID.into();
    late.input = ClaudeHookInput::SessionEnd {
        common: common(),
        reason: SessionEndReason::Other,
    };
    let scenario = scenario(HookAvailability::Enabled, vec![first.clone(), first, late]);
    let mut adapter = RecordingAdapter::default();
    assert_eq!(
        execute(&scenario, &mut adapter).unwrap(),
        ScenarioOutcome::Delivered {
            frame_count: OUT_OF_ORDER_FRAME_COUNT
        }
    );
    assert_eq!(adapter.frames, scenario.frames);
}

#[test]
fn missing_disabled_and_error_hooks_are_explicit_outcomes() {
    for availability in [HookAvailability::Missing, HookAvailability::DisableAllHooks] {
        let scenario = scenario(availability, vec![start_frame()]);
        let mut adapter = RecordingAdapter::default();
        assert_eq!(
            execute(&scenario, &mut adapter).unwrap(),
            ScenarioOutcome::Suppressed(availability)
        );
        assert!(adapter.frames.is_empty());
    }
    let mut adapter = RecordingAdapter::default();
    let error = execute(
        &scenario(HookAvailability::Error, vec![start_frame()]),
        &mut adapter,
    )
    .unwrap_err();
    assert_eq!(error.to_string(), SCENARIO_FORCED_ERROR);
    assert!(adapter.frames.is_empty());
}

#[test]
fn every_frame_is_validated_before_delivery_starts() {
    let valid = start_frame();
    let mut invalid = valid.clone();
    invalid.event_id = String::new();
    let mut adapter = RecordingAdapter::default();
    assert!(
        execute(
            &scenario(HookAvailability::Enabled, vec![valid, invalid]),
            &mut adapter,
        )
        .is_err()
    );
    assert!(adapter.frames.is_empty());
}

#[test]
fn json_line_adapter_flushes_independent_parseable_frames() {
    let mut bytes = Vec::new();
    let frame = start_frame();
    {
        let mut adapter = JsonLineHookAdapter::new(&mut bytes);
        adapter.deliver(&frame).unwrap();
        adapter.deliver(&frame).unwrap();
    }
    assert_eq!(
        bytes
            .iter()
            .filter(|byte| **byte == JSON_LINE_ENDING)
            .count(),
        JSON_LINE_FRAME_COUNT
    );
    let decoded: Vec<ClaudeHookFrame> = std::str::from_utf8(&bytes)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(decoded, [frame.clone(), frame]);
}

#[derive(Default)]
struct RecordingAdapter {
    frames: Vec<ClaudeHookFrame>,
}

impl ClaudeHookAdapter for RecordingAdapter {
    fn deliver(&mut self, frame: &ClaudeHookFrame) -> AdapterResult {
        self.frames.push(frame.clone());
        Ok(())
    }
}

fn scenario(availability: HookAvailability, frames: Vec<ClaudeHookFrame>) -> ClaudeScenario {
    ClaudeScenario {
        schema_version: SCENARIO_SCHEMA_VERSION,
        hook_availability: availability,
        frames,
    }
}

fn start_frame() -> ClaudeHookFrame {
    ClaudeHookFrame {
        schema_version: WIRE_SCHEMA_VERSION,
        event_id: EVENT_ID.into(),
        authority: authority(),
        input: ClaudeHookInput::SessionStart {
            common: common(),
            source: SessionStartSource::Startup,
        },
    }
}

fn authority() -> HookAuthority {
    HookAuthority {
        terminal_id: YOLO_SESSION_ID.into(),
        runtime_epoch: RUNTIME_EPOCH.into(),
        invocation_id: INVOCATION_ID.into(),
        hook_token: HOOK_TOKEN.into(),
        worker: identity(WORKER_PID, None),
        shell: identity(SHELL_PID, Some(WORKER_PID)),
        agent: identity(AGENT_PID, Some(SHELL_PID)),
        hook: identity(HOOK_PID, Some(AGENT_PID)),
    }
}

fn identity(pid: u32, parent_pid: Option<u32>) -> ProcessIdentity {
    ProcessIdentity {
        pid,
        start_time: START_TIME_BASE + u64::from(pid),
        parent_pid,
    }
}

fn common() -> CommonHookInput {
    CommonHookInput {
        session_id: AGENT_SESSION_ID.into(),
        transcript_path: PathBuf::from(TEST_TRANSCRIPT),
        cwd: PathBuf::from(TEST_CWD),
    }
}

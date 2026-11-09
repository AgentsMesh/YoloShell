use crate::claude_scenario::HookAvailability;
use crate::claude_wire::{ClaudeHookInput, SessionStartSource};
use crate::fake_claude_constants::DATA_HOME_DIRECTORY;
use crate::fake_claude_process::run;
use crate::fake_claude_scenarios::*;

const RESUME_ARGUMENT: &str = yolo_runtime::agent::claude::constants::RESUME_ARGUMENT;
const DUPLICATE_FRAME_INDEX: usize = 10;
const DUPLICATE_REPLAY_INDEX: usize = 11;
const LATE_END_INDEX: usize = 12;
const OUT_OF_ORDER_INDEX: usize = 13;
const FIRST_FRAME_INDEX: usize = 0;
const PROJECT_DIRECTORY: &str = "project";
const SHARED_PROJECT_DIRECTORY: &str = "shared-project";
const FIRST_DATA_HOME_DIRECTORY: &str = "claude-one";
const SECOND_DATA_HOME_DIRECTORY: &str = "claude-two";

#[test]
fn headless_process_emits_the_complete_claude_lifecycle_in_wire_order() {
    let directory = tempfile::tempdir().unwrap();
    let cwd = directory.path().join(PROJECT_DIRECTORY);
    let data_home = directory.path().join(DATA_HOME_DIRECTORY);
    let scenario = lifecycle(&cwd, &data_home);
    let result = run(&scenario, &[]);
    assert!(result.status.success(), "{}", result.stderr);
    assert_eq!(result.frames, scenario.frames);
    let fresh_id = &result.frames[FIRST_FRAME_INDEX].input.common().session_id;
    assert_eq!(fresh_id, FRESH_SESSION_ID);
    assert_eq!(
        uuid::Uuid::parse_str(fresh_id)
            .unwrap()
            .hyphenated()
            .to_string(),
        *fresh_id
    );
    assert_eq!(
        result.frames[DUPLICATE_FRAME_INDEX],
        result.frames[DUPLICATE_REPLAY_INDEX]
    );
    assert_eq!(
        result.frames[LATE_END_INDEX].input.common().session_id,
        FRESH_SESSION_ID
    );
    assert_eq!(
        result.frames[OUT_OF_ORDER_INDEX].input.common().session_id,
        CLEARED_SESSION_ID
    );
    let sources: Vec<_> = result
        .frames
        .iter()
        .filter_map(|frame| match frame.input {
            ClaudeHookInput::SessionStart { source, .. } => Some(source),
            _ => None,
        })
        .collect();
    assert_eq!(
        sources,
        [
            SessionStartSource::Startup,
            SessionStartSource::Clear,
            SessionStartSource::Compact,
            SessionStartSource::Fork,
            SessionStartSource::Fork,
        ]
    );
}

#[test]
fn resume_invocation_requires_and_emits_the_exact_canonical_session_id() {
    let directory = tempfile::tempdir().unwrap();
    let scenario = resume(
        directory.path(),
        &directory.path().join(DATA_HOME_DIRECTORY),
    );
    let result = run(&scenario, &[RESUME_ARGUMENT, RESUMED_SESSION_ID]);
    assert!(result.status.success(), "{}", result.stderr);
    assert_eq!(result.frames, scenario.frames);
    assert_eq!(
        result.frames[FIRST_FRAME_INDEX].input.common().session_id,
        RESUMED_SESSION_ID
    );
    assert_eq!(
        uuid::Uuid::parse_str(RESUMED_SESSION_ID)
            .unwrap()
            .hyphenated()
            .to_string(),
        RESUMED_SESSION_ID
    );
    let mismatch = run(&scenario, &[RESUME_ARGUMENT, FRESH_SESSION_ID]);
    assert!(!mismatch.status.success());
    assert!(mismatch.frames.is_empty());
}

#[test]
fn missing_disabled_and_error_hooks_never_fabricate_delivery() {
    for availability in [HookAvailability::Missing, HookAvailability::DisableAllHooks] {
        let scenario = suppressed(availability);
        let result = run(&scenario, &[]);
        assert!(result.status.success(), "{}", result.stderr);
        assert!(result.frames.is_empty());
    }
    let directory = tempfile::tempdir().unwrap();
    let result = run(
        &error(
            directory.path(),
            &directory.path().join(DATA_HOME_DIRECTORY),
        ),
        &[],
    );
    assert!(!result.status.success());
    assert!(result.frames.is_empty());
}

#[test]
fn two_headless_processes_in_one_cwd_keep_distinct_claude_sessions() {
    let directory = tempfile::tempdir().unwrap();
    let cwd = directory.path().join(SHARED_PROJECT_DIRECTORY);
    let first = lifecycle(&cwd, &directory.path().join(FIRST_DATA_HOME_DIRECTORY));
    let second = resume(&cwd, &directory.path().join(SECOND_DATA_HOME_DIRECTORY));
    let (first_result, second_result) = std::thread::scope(|scope| {
        let first = scope.spawn(|| run(&first, &[]));
        let second = scope.spawn(|| run(&second, &[RESUME_ARGUMENT, RESUMED_SESSION_ID]));
        (first.join().unwrap(), second.join().unwrap())
    });
    assert!(first_result.status.success(), "{}", first_result.stderr);
    assert!(second_result.status.success(), "{}", second_result.stderr);
    assert!(
        first_result
            .frames
            .iter()
            .all(|frame| frame.input.common().session_id != RESUMED_SESSION_ID)
    );
    assert!(
        second_result
            .frames
            .iter()
            .all(|frame| frame.input.common().session_id == RESUMED_SESSION_ID)
    );
}

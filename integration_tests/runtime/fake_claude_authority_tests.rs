use crate::claude_wire::{ClaudeHookFrame, ClaudeHookInput, SessionStartSource};
use crate::fake_claude_authority_contract::{
    ActivityState, ClaudeHookEndpoint, HookDisposition, HookFailure,
};
use crate::fake_claude_constants::{DATA_HOME_DIRECTORY, PROCESS_START_INCREMENT};
use crate::fake_claude_process::run;
use crate::fake_claude_reference_endpoint::ReferenceEndpoint;
use crate::fake_claude_reference_types::{GENERATION_INCREMENT, INITIAL_GENERATION};
use crate::fake_claude_scenarios::{RESUMED_SESSION_ID, lifecycle, resume};
use std::fs;
use std::path::Path;

const STATE_FILE_NAME: &str = "authority.json";
const ROTATED_RUNTIME_EPOCH: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const ROTATED_INVOCATION_ID: &str = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
const ROTATED_HOOK_TOKEN: &str = "cccccccc-cccc-4ccc-8ccc-cccccccccccc";
const ROTATED_EVENT_ID: &str = "dddddddd-dddd-4ddd-8ddd-dddddddddddd";
const FOLLOWUP_EVENT_ID: &str = "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee";
const PARTIAL_ROTATION_EVENT_ID: &str = "fddddddd-dddd-4ddd-8ddd-dddddddddddd";
const PROCESS_ID_INCREMENT: u32 = 100;
const DUPLICATE_FRAME_INDEX: usize = 11;
const LATE_END_FRAME_INDEX: usize = 12;
const OUT_OF_ORDER_FRAME_INDEX: usize = 13;
const APPLIED_LIFECYCLE_FRAME_COUNT: usize = 11;
const LIFECYCLE_GENERATION_ADVANCES: u64 = 2;
const INITIAL_FRAME_INDEX: usize = 0;
const TRANSCRIPT_CONTENTS: &[u8] = b"{}\n";
const PROJECT_DIRECTORY: &str = "project";

#[test]
fn lifecycle_process_drives_one_durable_generation_authority() {
    let directory = tempfile::tempdir().unwrap();
    let data_home = directory.path().join(DATA_HOME_DIRECTORY);
    let scenario = lifecycle(&directory.path().join(PROJECT_DIRECTORY), &data_home);
    materialize(&scenario.frames);
    let result = run(&scenario, &[]);
    assert!(result.status.success(), "{}", result.stderr);
    let mut endpoint = endpoint(directory.path(), &data_home);
    let dispositions: Vec<_> = result
        .frames
        .iter()
        .map(|frame| endpoint.apply(frame).unwrap())
        .collect();
    assert_eq!(dispositions[DUPLICATE_FRAME_INDEX], HookDisposition::Replay);
    assert_eq!(dispositions[LATE_END_FRAME_INDEX], HookDisposition::Stale);
    assert_eq!(
        dispositions[OUT_OF_ORDER_FRAME_INDEX],
        HookDisposition::Stale
    );
    assert!(
        dispositions[..APPLIED_LIFECYCLE_FRAME_COUNT]
            .iter()
            .all(|value| *value == HookDisposition::Applied)
    );
    let view = endpoint.view().unwrap();
    assert_eq!(
        view.generation,
        INITIAL_GENERATION + LIFECYCLE_GENERATION_ADVANCES * GENERATION_INCREMENT
    );
    assert_eq!(view.activity, ActivityState::Running);
}

#[test]
fn app_host_and_worker_crashes_preserve_binding_but_fence_runtime_authority() {
    let directory = tempfile::tempdir().unwrap();
    let data_home = directory.path().join(DATA_HOME_DIRECTORY);
    let scenario = resume(directory.path(), &data_home);
    materialize(&scenario.frames);
    let state_path = directory.path().join(STATE_FILE_NAME);
    let mut endpoint = ReferenceEndpoint::open(&state_path, &data_home).unwrap();
    assert_eq!(
        endpoint
            .apply(&scenario.frames[INITIAL_FRAME_INDEX])
            .unwrap(),
        HookDisposition::Applied
    );
    let initial = endpoint.view().unwrap();

    drop(endpoint);
    let mut after_app = ReferenceEndpoint::open(&state_path, &data_home).unwrap();
    assert_eq!(after_app.view(), Some(initial.clone()));
    assert_eq!(
        after_app
            .apply(&scenario.frames[INITIAL_FRAME_INDEX])
            .unwrap(),
        HookDisposition::Replay
    );
    drop(after_app);
    let mut after_host = ReferenceEndpoint::open(&state_path, &data_home).unwrap();
    assert_eq!(after_host.view(), Some(initial.clone()));

    after_host.retire_runtime().unwrap();
    drop(after_host);
    let mut after_worker = ReferenceEndpoint::open(&state_path, &data_home).unwrap();
    let retired = after_worker.view().unwrap();
    assert_eq!(retired.provider_session_id, RESUMED_SESSION_ID);
    assert_eq!(retired.activity, ActivityState::Unknown);
    assert!(!retired.runtime_active);
    assert_eq!(
        after_worker
            .apply(&followup(&scenario.frames[INITIAL_FRAME_INDEX]))
            .unwrap(),
        HookDisposition::Rejected
    );
    let mut partial = scenario.frames[INITIAL_FRAME_INDEX].clone();
    partial.event_id = PARTIAL_ROTATION_EVENT_ID.into();
    partial.authority.hook_token = ROTATED_HOOK_TOKEN.into();
    assert_eq!(
        after_worker.resume_runtime(&partial).unwrap(),
        HookDisposition::Rejected
    );
    let resumed = rotated_resume(&scenario.frames[INITIAL_FRAME_INDEX]);
    assert_eq!(
        after_worker.resume_runtime(&resumed).unwrap(),
        HookDisposition::Applied
    );
    let active = after_worker.view().unwrap();
    assert!(active.runtime_active);
    assert_eq!(active.generation, initial.generation + GENERATION_INCREMENT);
}

#[test]
fn missing_error_and_disabled_hooks_invalidate_recovery_authority() {
    for failure in [
        HookFailure::Missing,
        HookFailure::Error,
        HookFailure::Disabled,
    ] {
        let directory = tempfile::tempdir().unwrap();
        let data_home = directory.path().join(DATA_HOME_DIRECTORY);
        let scenario = resume(directory.path(), &data_home);
        materialize(&scenario.frames);
        let mut endpoint = endpoint(directory.path(), &data_home);
        endpoint
            .apply(&scenario.frames[INITIAL_FRAME_INDEX])
            .unwrap();
        endpoint.mark_unavailable(failure).unwrap();
        let view = endpoint.view().unwrap();
        assert_eq!(view.provider_session_id, RESUMED_SESSION_ID);
        assert_eq!(view.activity, ActivityState::Unknown);
        assert!(!view.runtime_active);
    }
}

fn endpoint(root: &Path, data_home: &Path) -> ReferenceEndpoint {
    ReferenceEndpoint::open(&root.join(STATE_FILE_NAME), data_home).unwrap()
}

pub(crate) fn materialize(frames: &[ClaudeHookFrame]) {
    for frame in frames {
        let path = &frame.input.common().transcript_path;
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        if !path.exists() {
            fs::write(path, TRANSCRIPT_CONTENTS).unwrap();
        }
    }
}

pub(crate) fn followup(first: &ClaudeHookFrame) -> ClaudeHookFrame {
    let mut next = first.clone();
    next.event_id = FOLLOWUP_EVENT_ID.into();
    next.input = ClaudeHookInput::Stop {
        common: first.input.common().clone(),
        stop_hook_active: false,
        last_assistant_message: String::new(),
    };
    next
}

fn rotated_resume(first: &ClaudeHookFrame) -> ClaudeHookFrame {
    let mut next = first.clone();
    next.event_id = ROTATED_EVENT_ID.into();
    next.authority.runtime_epoch = ROTATED_RUNTIME_EPOCH.into();
    next.authority.invocation_id = ROTATED_INVOCATION_ID.into();
    next.authority.hook_token = ROTATED_HOOK_TOKEN.into();
    rotate_processes(&mut next);
    next.input = ClaudeHookInput::SessionStart {
        common: first.input.common().clone(),
        source: SessionStartSource::Resume,
    };
    next
}

fn rotate_processes(frame: &mut ClaudeHookFrame) {
    let identities = [
        &mut frame.authority.worker,
        &mut frame.authority.shell,
        &mut frame.authority.agent,
        &mut frame.authority.hook,
    ];
    for identity in identities {
        identity.pid += PROCESS_ID_INCREMENT;
        identity.start_time += PROCESS_START_INCREMENT;
        identity.parent_pid = identity.parent_pid.map(|pid| pid + PROCESS_ID_INCREMENT);
    }
}

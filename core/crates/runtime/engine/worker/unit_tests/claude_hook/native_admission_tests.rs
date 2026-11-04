use super::*;
use crate::claude_hooks::ClaudeProcessIdentity;
use std::path::Path;

const SHELL_PID: u32 = 54_001;
const SHELL_START: u64 = 54_002;
const AGENT_PID: u32 = 54_003;
const AGENT_START: u64 = 54_004;
const PEER_PID: u32 = 54_005;
const PEER_START: u64 = 54_006;
const WRAPPER_PID: u32 = 54_007;
const WRAPPER_START: u64 = 54_008;
const GROUP: i32 = 54_003;
const OTHER_GROUP: i32 = 54_100;
const HOOK: &str = "/Applications/Yolo.app/Contents/MacOS/yolo-host";
const CLAUDE: &str = "/opt/homebrew/bin/claude";
const WRAPPER: &str = "/bin/sh";

fn fixture() -> (ProcessIdentity, ClaudeNativeScope, Vec<ObservedProcess>) {
    let scope = ClaudeNativeScope {
        terminal_id: uuid::Uuid::new_v4(),
        runtime_epoch: uuid::Uuid::new_v4(),
        agent: ClaudeProcessIdentity {
            pid: AGENT_PID,
            start_time: AGENT_START,
        },
    };
    let process = |pid, start_time, parent_pid, executable: &str| ObservedProcess {
        pid,
        start_time,
        parent_pid,
        executable: executable.to_owned(),
        argv: vec![executable.to_owned()],
        process_group: GROUP,
        foreground_group: GROUP,
        cwd: None,
        evidence_paths: None,
        environment: Some(std::collections::BTreeMap::from([
            (
                crate::agent::constants::YOLO_TERMINAL_ID_ENV.to_owned(),
                scope.terminal_id.to_string(),
            ),
            (
                crate::agent::constants::YOLO_RUNTIME_EPOCH_ENV.to_owned(),
                scope.runtime_epoch.to_string(),
            ),
        ])),
    };
    let mut processes = vec![
        process(PEER_PID, PEER_START, WRAPPER_PID, HOOK),
        process(WRAPPER_PID, WRAPPER_START, AGENT_PID, WRAPPER),
        process(AGENT_PID, AGENT_START, SHELL_PID, CLAUDE),
    ];
    // Claude runs hooks as background children with their own process group
    // and no controlling terminal; only the agent owns the foreground group.
    let peer = processes
        .iter_mut()
        .find(|item| item.pid == PEER_PID)
        .unwrap();
    peer.process_group = PEER_PID as i32;
    peer.foreground_group = -1;
    (
        ProcessIdentity {
            pid: SHELL_PID,
            start_time: SHELL_START,
        },
        scope,
        processes,
    )
}

#[test]
fn background_hook_child_of_a_foreground_agent_is_admitted() {
    let (shell, scope, processes) = fixture();
    let peer = processes
        .iter()
        .find(|item| item.pid == PEER_PID)
        .unwrap();
    assert!(!peer.is_foreground());
    assert!(select(PEER_PID, shell, scope, Path::new(HOOK), &processes).is_ok());
}

#[test]
fn a_hook_that_does_not_descend_from_the_agent_is_rejected() {
    let (shell, scope, mut processes) = fixture();
    processes
        .iter_mut()
        .find(|item| item.pid == PEER_PID)
        .unwrap()
        .parent_pid = SHELL_PID;
    assert!(select(PEER_PID, shell, scope, Path::new(HOOK), &processes).is_err());
}

#[test]
fn native_shell_callback_is_admitted_without_launcher_context() {
    let (shell, scope, processes) = fixture();
    let admitted = select(PEER_PID, shell, scope, Path::new(HOOK), &processes).unwrap();
    assert_eq!(admitted.agent.pid, AGENT_PID);
    assert_eq!(admitted.peer.start_time, PEER_START);
}

#[test]
fn pid_reuse_foreign_epoch_and_background_agent_are_rejected() {
    let (shell, mut scope, mut processes) = fixture();
    scope.agent.start_time += u64::from(u8::MAX);
    assert!(select(PEER_PID, shell, scope, Path::new(HOOK), &processes).is_err());
    scope.agent.start_time = AGENT_START;
    scope.runtime_epoch = uuid::Uuid::new_v4();
    assert!(select(PEER_PID, shell, scope, Path::new(HOOK), &processes).is_err());
    let (shell, scope, _) = fixture();
    processes
        .iter_mut()
        .find(|item| item.pid == AGENT_PID)
        .unwrap()
        .foreground_group = OTHER_GROUP;
    assert!(select(PEER_PID, shell, scope, Path::new(HOOK), &processes).is_err());
}

#[test]
fn nested_claude_and_arbitrary_callback_wrapper_are_rejected() {
    let (shell, scope, mut processes) = fixture();
    let wrapper = processes
        .iter_mut()
        .find(|item| item.pid == WRAPPER_PID)
        .unwrap();
    wrapper.executable = CLAUDE.to_owned();
    wrapper.argv = vec![CLAUDE.to_owned()];
    assert!(select(PEER_PID, shell, scope, Path::new(HOOK), &processes).is_err());
    wrapper_reset(&mut processes);
    assert!(select(PEER_PID, shell, scope, Path::new(HOOK), &processes).is_err());
}

#[test]
fn duplicate_pid_rows_are_rejected_even_when_identical() {
    for pid in [PEER_PID, WRAPPER_PID, AGENT_PID] {
        let (shell, scope, mut processes) = fixture();
        let duplicate = processes
            .iter()
            .find(|item| item.pid == pid)
            .unwrap()
            .clone();
        processes.push(duplicate);
        assert!(select(PEER_PID, shell, scope, Path::new(HOOK), &processes).is_err());
    }
}

#[test]
fn contradictory_duplicate_pid_rows_are_rejected_in_either_order() {
    for pid in [PEER_PID, WRAPPER_PID, AGENT_PID] {
        let (shell, scope, mut processes) = fixture();
        let mut duplicate = processes
            .iter()
            .find(|item| item.pid == pid)
            .unwrap()
            .clone();
        duplicate.foreground_group = OTHER_GROUP;
        processes.push(duplicate);
        assert!(select(PEER_PID, shell, scope, Path::new(HOOK), &processes).is_err());
        processes.reverse();
        assert!(select(PEER_PID, shell, scope, Path::new(HOOK), &processes).is_err());
    }
}

fn wrapper_reset(processes: &mut [ObservedProcess]) {
    let wrapper = processes
        .iter_mut()
        .find(|item| item.pid == WRAPPER_PID)
        .unwrap();
    wrapper.executable = HOOK.to_owned();
    wrapper.argv = vec![HOOK.to_owned()];
}

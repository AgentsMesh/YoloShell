use super::super::admission::peer_is_direct_hook;
use crate::agent::ObservedProcess;

const SHELL_PID: u32 = 41_000;
const AGENT_PID: u32 = 41_001;
const HELPER_PID: u32 = 41_002;
const WRAPPER_PID: u32 = 41_003;
const FOREIGN_PID: u32 = 41_004;
const AGENT_START: u64 = 51_001;
const HELPER_START: u64 = 51_002;
const WRAPPER_START: u64 = 51_003;
const FOREIGN_START: u64 = 51_004;
const PROCESS_GROUP: i32 = 61_001;
const IDENTITY_DELTA: u64 = 1;
const PROCESS_GROUP_DELTA: i32 = 1;
const TEST_EXECUTABLE: &str = "/test/codex-hook";

#[test]
fn only_the_exact_direct_hook_generation_is_admitted() {
    let agent = process(AGENT_PID, AGENT_START, SHELL_PID);
    let wrapper = process(WRAPPER_PID, WRAPPER_START, AGENT_PID);
    let helper = process(HELPER_PID, HELPER_START, WRAPPER_PID);
    let processes = [agent, wrapper, helper];

    assert!(peer_is_direct_hook(
        process_identity(WRAPPER_PID, WRAPPER_START),
        agent_identity(),
        std::path::Path::new(TEST_EXECUTABLE),
        &processes,
    ));
    assert!(!peer_is_direct_hook(
        agent_identity(),
        agent_identity(),
        std::path::Path::new(TEST_EXECUTABLE),
        &processes,
    ));
    assert!(!peer_is_direct_hook(
        process_identity(HELPER_PID, HELPER_START),
        agent_identity(),
        std::path::Path::new(TEST_EXECUTABLE),
        &processes,
    ));
}

#[test]
fn reused_and_zero_start_peer_generations_are_rejected() {
    let agent = process(AGENT_PID, AGENT_START, SHELL_PID);
    let hook = process(WRAPPER_PID, WRAPPER_START, AGENT_PID);
    let processes = [agent, hook];

    assert!(!peer_is_direct_hook(
        process_identity(WRAPPER_PID, WRAPPER_START.saturating_add(IDENTITY_DELTA),),
        agent_identity(),
        std::path::Path::new(TEST_EXECUTABLE),
        &processes,
    ));
    assert!(!peer_is_direct_hook(
        process_identity(WRAPPER_PID, u64::default()),
        agent_identity(),
        std::path::Path::new(TEST_EXECUTABLE),
        &processes,
    ));
}

#[test]
fn peer_must_reach_the_exact_agent_through_a_unique_foreground_lineage() {
    let agent = process(AGENT_PID, AGENT_START, SHELL_PID);
    let helper = process(HELPER_PID, HELPER_START, FOREIGN_PID);
    assert!(!peer_is_direct_hook(
        process_identity(HELPER_PID, HELPER_START),
        agent_identity(),
        std::path::Path::new(TEST_EXECUTABLE),
        &[agent.clone(), helper.clone()]
    ));
    assert!(!peer_is_direct_hook(
        process_identity(FOREIGN_PID, FOREIGN_START),
        agent_identity(),
        std::path::Path::new(TEST_EXECUTABLE),
        &[agent.clone(), helper.clone()]
    ));

    let mut background = helper.clone();
    background.foreground_group = PROCESS_GROUP.saturating_add(PROCESS_GROUP_DELTA);
    assert!(!peer_is_direct_hook(
        process_identity(HELPER_PID, HELPER_START),
        agent_identity(),
        std::path::Path::new(TEST_EXECUTABLE),
        &[agent.clone(), background]
    ));

    let mut reused = agent.clone();
    reused.start_time = AGENT_START.saturating_add(IDENTITY_DELTA);
    assert!(!peer_is_direct_hook(
        agent_identity(),
        agent_identity(),
        std::path::Path::new(TEST_EXECUTABLE),
        &[reused],
    ));

    assert!(!peer_is_direct_hook(
        agent_identity(),
        agent_identity(),
        std::path::Path::new(TEST_EXECUTABLE),
        &[agent.clone(), agent]
    ));
}

#[test]
fn cyclic_helper_lineage_is_rejected() {
    let agent = process(AGENT_PID, AGENT_START, SHELL_PID);
    let helper = process(HELPER_PID, HELPER_START, WRAPPER_PID);
    let wrapper = process(WRAPPER_PID, WRAPPER_START, HELPER_PID);

    assert!(!peer_is_direct_hook(
        process_identity(HELPER_PID, HELPER_START),
        agent_identity(),
        std::path::Path::new(TEST_EXECUTABLE),
        &[agent, helper, wrapper]
    ));
}

fn agent_identity() -> crate::process::ProcessIdentity {
    process_identity(AGENT_PID, AGENT_START)
}

fn process_identity(pid: u32, start_time: u64) -> crate::process::ProcessIdentity {
    crate::process::ProcessIdentity { pid, start_time }
}

fn process(pid: u32, start_time: u64, parent_pid: u32) -> ObservedProcess {
    ObservedProcess {
        pid,
        start_time,
        parent_pid,
        process_group: PROCESS_GROUP,
        foreground_group: PROCESS_GROUP,
        executable: TEST_EXECUTABLE.to_owned(),
        argv: Vec::new(),
        environment: None,
        cwd: None,
        evidence_paths: None,
    }
}

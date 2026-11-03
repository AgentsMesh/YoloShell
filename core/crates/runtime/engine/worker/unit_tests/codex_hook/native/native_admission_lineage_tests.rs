use super::test_support::*;

#[test]
fn native_callback_binds_agent_identity_not_helper_identity() {
    let (agent, _) = capture(&direct()).unwrap();
    assert_eq!(agent.pid, AGENT_PID);
    assert_ne!(agent.pid, CALLBACK_PID);
}

#[test]
fn native_command_shell_intermediary_keeps_the_same_agent() {
    let chain = vec![
        process(AGENT_PID, SHELL_PID, AGENT_EXECUTABLE),
        process(WRAPPER_PID, AGENT_PID, COMMAND_SHELL),
        process(CALLBACK_PID, WRAPPER_PID, HOOK_EXECUTABLE),
    ];
    assert_eq!(capture(&chain).unwrap().0.pid, AGENT_PID);
}

#[test]
fn callback_outside_native_agent_cannot_register() {
    let chain = vec![process(CALLBACK_PID, SHELL_PID, HOOK_EXECUTABLE)];
    assert!(capture(&chain).is_err());
}

#[test]
fn changed_callback_start_time_cannot_register() {
    let mut chain = direct();
    chain
        .iter_mut()
        .find(|process| process.pid == CALLBACK_PID)
        .unwrap()
        .start_time += IDENTITY_INCREMENT;
    assert!(capture(&chain).is_err());
}

#[test]
fn duplicate_pid_observation_cannot_authorize_any_branch() {
    let mut chain = direct();
    chain.push(chain.first().unwrap().clone());
    assert!(capture(&chain).is_err());
}

#[test]
fn ancestry_cycle_cannot_be_treated_as_a_terminal_shell_descendant() {
    let mut chain = direct();
    chain
        .iter_mut()
        .find(|process| process.pid == AGENT_PID)
        .unwrap()
        .parent_pid = CALLBACK_PID;
    assert!(capture(&chain).is_err());
}

#[test]
fn cross_terminal_foreground_group_cannot_register() {
    let mut chain = direct();
    let agent = chain
        .iter_mut()
        .find(|process| process.pid == AGENT_PID)
        .unwrap();
    agent.foreground_group += GROUP_INCREMENT;
    agent.process_group += GROUP_INCREMENT;
    assert!(capture(&chain).is_err());
}

#[test]
fn replaced_callback_executable_cannot_register() {
    let mut chain = direct();
    chain
        .iter_mut()
        .find(|process| process.pid == CALLBACK_PID)
        .unwrap()
        .executable = FOREIGN_EXECUTABLE.to_owned();
    assert!(capture(&chain).is_err());
}

#[test]
fn arbitrary_tool_process_cannot_forward_a_callback_as_native_hook() {
    let chain = vec![
        process(AGENT_PID, SHELL_PID, AGENT_EXECUTABLE),
        process(WRAPPER_PID, AGENT_PID, FOREIGN_EXECUTABLE),
        process(CALLBACK_PID, WRAPPER_PID, HOOK_EXECUTABLE),
    ];
    assert!(capture(&chain).is_err());
}

#[test]
fn missing_parent_and_background_agent_are_rejected() {
    assert!(capture(&[process(CALLBACK_PID, AGENT_PID, HOOK_EXECUTABLE)]).is_err());
    let mut chain = direct();
    chain
        .iter_mut()
        .find(|process| process.pid == AGENT_PID)
        .unwrap()
        .foreground_group = i32::default();
    assert!(capture(&chain).is_err());
}

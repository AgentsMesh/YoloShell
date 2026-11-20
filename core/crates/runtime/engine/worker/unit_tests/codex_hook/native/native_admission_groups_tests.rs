use super::test_support::*;

fn command_group() -> Vec<super::ObservedProcess> {
    let mut chain = vec![
        process(AGENT_PID, SHELL_PID, AGENT_EXECUTABLE),
        process(WRAPPER_PID, AGENT_PID, COMMAND_SHELL),
        process(CALLBACK_PID, WRAPPER_PID, HOOK_EXECUTABLE),
    ];
    for process in chain.iter_mut().filter(|process| process.pid != AGENT_PID) {
        process.process_group = i32::try_from(WRAPPER_PID).unwrap();
    }
    chain
}

#[test]
fn codex_command_hook_group_keeps_foreground_provider_authority() {
    let (agent, lineage) = capture(&command_group()).unwrap();
    assert_eq!(agent.pid, AGENT_PID);
    assert!(agent.is_foreground());
    assert!(!lineage.first().unwrap().is_foreground());
}

#[test]
fn shell_exec_into_callback_can_remain_its_own_group_leader() {
    let mut chain = direct();
    chain.last_mut().unwrap().process_group = i32::try_from(CALLBACK_PID).unwrap();
    assert_eq!(capture(&chain).unwrap().0.pid, AGENT_PID);
}

#[test]
fn unrelated_group_and_changed_intermediary_group_are_rejected() {
    let mut foreign_group = command_group();
    for process in foreign_group
        .iter_mut()
        .filter(|process| process.pid != AGENT_PID)
    {
        process.process_group += GROUP_INCREMENT;
    }
    assert!(capture(&foreign_group).is_err());
    let mut changed_shell = command_group();
    changed_shell
        .iter_mut()
        .find(|process| process.pid == WRAPPER_PID)
        .unwrap()
        .process_group = FOREGROUND_GROUP;
    assert!(capture(&changed_shell).is_err());
}

#[test]
fn detached_tty_background_provider_and_untrusted_prefix_still_fail() {
    let mut detached = command_group();
    detached.last_mut().unwrap().foreground_group = i32::default();
    assert!(capture(&detached).is_err());
    let mut background = command_group();
    background.first_mut().unwrap().process_group += GROUP_INCREMENT;
    assert!(capture(&background).is_err());
    let mut foreign_shell = command_group();
    foreign_shell
        .iter_mut()
        .find(|process| process.pid == WRAPPER_PID)
        .unwrap()
        .executable = FOREIGN_EXECUTABLE.to_owned();
    assert!(capture(&foreign_shell).is_err());
}

#[test]
fn exact_admission_enforces_the_same_group_boundary() {
    let chain = command_group();
    let result = super::capture_exact(
        identity(CALLBACK_PID),
        identity(SHELL_PID),
        std::path::Path::new(HOOK_EXECUTABLE),
        identity(AGENT_PID),
        &chain,
    )
    .unwrap();
    assert_eq!(result.0.pid, AGENT_PID);
}

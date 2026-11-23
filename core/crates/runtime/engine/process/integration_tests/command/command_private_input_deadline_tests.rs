#[test]
fn nonreading_child_hits_delivery_deadline_and_is_reaped() {
    let root = tempfile::tempdir().unwrap();
    let pid_file = root.path().join(CHILD_PID_FILE);
    let input = PRIVATE_INPUT
        .iter()
        .copied()
        .cycle()
        .take(PIPE_OVERFLOW_BYTES)
        .collect::<Vec<_>>();
    let mut command = std::process::Command::new(SHELL_EXECUTABLE);
    command.args([
        SHELL_COMMAND_ARGUMENT.into(),
        HOLD_WITHOUT_READING_SCRIPT.into(),
        SHELL_ARGUMENT_ZERO.into(),
        pid_file.as_os_str().to_owned(),
    ]);
    let mut child = super::spawn_detached_with_control_pipes(command).unwrap();
    let process_id = child.id();
    wait_until_pid_published(
        &pid_file,
        process_id,
        PRIVATE_INPUT_TEST_BUDGET,
        PRIVATE_INPUT_OBSERVATION_POLL,
    );

    let started = Instant::now();
    let error = super::write_detached_child_private_input(
        child.stdin.take().unwrap(),
        &input,
        started + PRIVATE_INPUT_TIMEOUT_BUDGET,
    )
    .unwrap_err();

    assert!(started.elapsed() < PRIVATE_INPUT_TEST_BUDGET);
    assert!(
        error
            .to_string()
            .contains(super::command_private_input::PRIVATE_INPUT_DEADLINE_CONTEXT)
    );
    super::terminate_detached_child(child).unwrap();
    wait_until_reaped(
        process_id,
        PRIVATE_INPUT_TEST_BUDGET,
        PRIVATE_INPUT_OBSERVATION_POLL,
    );
}

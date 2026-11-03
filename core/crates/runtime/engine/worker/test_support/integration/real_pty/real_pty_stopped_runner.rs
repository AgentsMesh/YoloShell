fn run_stopped_output_isolation(case: StoppedOutputCase) -> Result<()> {
    let _isolation = STOPPED_OUTPUT_ISOLATION_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let directory = tempfile::tempdir()?;
    let context = StoppedOutputContext {
        directory: directory.path().to_owned(),
        nonce: Uuid::new_v4().to_string(),
        case,
    };
    let mut socket_root = crate::paths::IsolatedTestSocketRoot::create()?;
    let mut command = std::process::Command::new(std::env::current_exe()?);
    command.args([
        BOOTSTRAP_TEST_EXACT_ARGUMENT,
        STOPPED_OUTPUT_ISOLATED_TEST,
        BOOTSTRAP_TEST_NOCAPTURE_ARGUMENT,
        crate::constants::TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
    ]);
    command.env(STOPPED_OUTPUT_ISOLATION_ENV, STOPPED_OUTPUT_ISOLATION_ROLE);
    command.env(STOPPED_OUTPUT_CASE_ENV, case.name());
    command.env(STOPPED_OUTPUT_CONTROL_ENV, &context.directory);
    command.env(STOPPED_OUTPUT_NONCE_ENV, &context.nonce);
    command.env(crate::paths::SOCKET_ROOT_ENVIRONMENT, socket_root.path());
    command.stdin(std::process::Stdio::null());
    let mut child = StoppedOutputChild::spawn(command, context.clone())?;
    let mut failures = Vec::new();
    let require_completion = match child.wait_for_exit() {
        Ok(status) if status.success() => true,
        Ok(status) => {
            failures.push(format!("{STOPPED_OUTPUT_CHILD_FAILURE_DETAIL}: {status}"));
            false
        }
        Err(error) => {
            failures.push(error.to_string());
            record_stopped_output_result(&mut failures, context.settle_root());
            record_stopped_output_result(&mut failures, child.terminate_and_reap().map(|_| ()));
            false
        }
    };
    record_stopped_output_result(
        &mut failures,
        context.verify(&child.live, require_completion),
    );
    if failures.is_empty() {
        record_stopped_output_result(&mut failures, context.assert_root_gone());
    } else {
        record_stopped_output_result(&mut failures, context.settle_root());
    }
    record_stopped_output_result(
        &mut failures,
        settle_worker_test_socket_namespaces(&socket_root),
    );
    record_stopped_output_result(&mut failures, socket_root.finalize());
    anyhow::ensure!(
        failures.is_empty(),
        failures.join(STOPPED_OUTPUT_FAILURE_SEPARATOR)
    );
    Ok(())
}

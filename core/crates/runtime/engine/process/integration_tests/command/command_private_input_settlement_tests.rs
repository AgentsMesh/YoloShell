use super::command_private_spawn::PRIVATE_INPUT_SETTLEMENT_CONTEXT;
use super::command_settlement::{
    PRIVATE_INPUT_POLL_CONTEXT, PrivateInputCleanupFailure, SettlementFailures,
};
use super::command_settlement_test_hook::{PollOverride, SettlementScript, install};

const SETTLEMENT_PID_FILE: &str = "private-input-settlement.pid";
const PID_THEN_HOLD_CLOSED_SCRIPT: &str =
    "printf '%s' \"$$\" > \"$1\"; exec 0<&-; while :; do :; done";
const SETTLEMENT_POLL_FAILURE: &str = "injected settlement poll failure";
const SETTLEMENT_TEST_BUDGET: Duration = Duration::from_secs(2);

#[test]
fn private_input_failure_preserves_settlement_context_and_exact_custody() {
    let root = tempfile::tempdir().unwrap();
    let pid_file = root.path().join(SETTLEMENT_PID_FILE);
    let input = PRIVATE_INPUT
        .iter()
        .copied()
        .cycle()
        .take(PIPE_OVERFLOW_BYTES)
        .collect::<Vec<_>>();
    let _script = install(SettlementScript::new([PollOverride::Failure(
        SETTLEMENT_POLL_FAILURE,
    )]));

    let error = spawn_detached_with_private_input(
        Path::new(SHELL_EXECUTABLE),
        &[
            SHELL_COMMAND_ARGUMENT.into(),
            PID_THEN_HOLD_CLOSED_SCRIPT.into(),
            SHELL_ARGUMENT_ZERO.into(),
            pid_file.as_os_str().to_owned(),
        ],
        DetachedStandardError::Null,
        &input,
        Instant::now() + SETTLEMENT_TEST_BUDGET,
    )
    .unwrap_err();
    let combined = error
        .downcast_ref::<PrivateInputCleanupFailure>()
        .unwrap();
    assert!(combined.input().chain().any(|cause| cause.is::<io::Error>()));
    assert!(
        combined
            .settlement()
            .downcast_ref::<SettlementFailures>()
            .is_some()
    );
    assert!(std::error::Error::source(combined).is_some());
    let detail = format!("{error:#}");
    assert!(detail.contains(PRIVATE_INPUT_SETTLEMENT_CONTEXT));
    assert!(detail.contains(PRIVATE_INPUT_POLL_CONTEXT));
    assert!(detail.contains(SETTLEMENT_POLL_FAILURE));
    let process_id = std::fs::read_to_string(pid_file)
        .unwrap()
        .parse::<u32>()
        .unwrap();
    drop(_script);

    wait_until_reaped(
        process_id,
        SETTLEMENT_TEST_BUDGET,
        PRIVATE_INPUT_OBSERVATION_POLL,
    );
}

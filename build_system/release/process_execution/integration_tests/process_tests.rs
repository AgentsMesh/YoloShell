use super::process_supervision_tests::Fixture;
use super::support::error_text;
use super::test_constants::{
    LONG_SLEEP_SECONDS, SLEEP_EXECUTABLE, SUCCESS_TIMEOUT_SECONDS, TEST_TIMEOUT_MILLISECONDS,
    YES_EXECUTABLE,
};
use std::time::Duration;

const ECHO_EXECUTABLE: &str = "/bin/echo";
const ECHO_ARGUMENT: &str = "bounded";
const TIMEOUT_DESCRIPTION: &str = "test child";
const OVERFLOW_DESCRIPTION: &str = "unbounded output";
const ECHO_DESCRIPTION: &str = "echo";

#[test]
fn bounded_process_is_killed_at_named_timeout() {
    let fixture = Fixture::new();
    let error = error_text(
        fixture
            .command(
                SLEEP_EXECUTABLE,
                vec![LONG_SLEEP_SECONDS.to_owned()],
                TIMEOUT_DESCRIPTION,
            )
            .with_timeout(Duration::from_millis(TEST_TIMEOUT_MILLISECONDS))
            .execute(),
    );
    assert!(error.contains(&format!("{TIMEOUT_DESCRIPTION} timed out")));
}

#[test]
fn generic_process_output_overflow_is_rejected() {
    let fixture = Fixture::new();
    let error = error_text(
        fixture
            .command(YES_EXECUTABLE, Vec::new(), OVERFLOW_DESCRIPTION)
            .with_timeout(Duration::from_secs(SUCCESS_TIMEOUT_SECONDS))
            .execute(),
    );
    assert!(error.contains("output exceeded the bounded limit"));
}

#[test]
fn bounded_process_collects_successful_output() {
    let fixture = Fixture::new();
    let output = fixture
        .command(
            ECHO_EXECUTABLE,
            vec![ECHO_ARGUMENT.to_owned()],
            ECHO_DESCRIPTION,
        )
        .with_timeout(Duration::from_secs(SUCCESS_TIMEOUT_SECONDS))
        .execute()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"bounded\n");
}

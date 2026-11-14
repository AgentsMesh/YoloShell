use super::*;
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

const ACTIVE_STATE: &str = "R";
const DEAD_STATE: &str = "X";
const FIXTURE_FIELDS_BEFORE_START: &str = "1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18";
const FIXTURE_PREFIX: &str = "42 (fixture command)";
const FIXTURE_START_TIME: u64 = 4_242;
const INVALID_STATE: &str = "Running";
const INVALID_START_TIME: &str = "invalid";
const LOWERCASE_DEAD_STATE: &str = "x";
const POLL_INTERVAL: Duration = Duration::from_millis(5);
const QUICK_EXIT_SCRIPT: &str = "exit 0";
const SHELL_COMMAND_ARGUMENT: &str = "-c";
const SHELL_EXECUTABLE: &str = "sh";
const STATE_WAIT_TIMEOUT: Duration = Duration::from_secs(2);
const ZOMBIE_STATE: &str = "Z";

#[test]
fn parser_classifies_active_and_inactive_states() {
    let cases = [
        (ACTIVE_STATE, true),
        (ZOMBIE_STATE, false),
        (DEAD_STATE, false),
        (LOWERCASE_DEAD_STATE, false),
    ];
    for (state, active) in cases {
        let parsed =
            parse_linux_process_stat(&fixture_stat(state, &FIXTURE_START_TIME.to_string()))
                .unwrap();
        assert_eq!(parsed.state.to_string(), state);
        assert_eq!(parsed.start_time, FIXTURE_START_TIME);
        assert_eq!(parsed.is_active(), active);
    }
}

#[test]
fn parser_rejects_each_malformed_stat_shape() {
    let cases = [
        ("42 (fixture", LINUX_MALFORMED_STAT_DETAIL),
        (FIXTURE_PREFIX, LINUX_PROCESS_STATE_MISSING_DETAIL),
        (
            &format!("{FIXTURE_PREFIX}  {FIXTURE_FIELDS_BEFORE_START} {FIXTURE_START_TIME}"),
            LINUX_PROCESS_STATE_EMPTY_DETAIL,
        ),
        (
            &fixture_stat(INVALID_STATE, &FIXTURE_START_TIME.to_string()),
            LINUX_PROCESS_STATE_INVALID_DETAIL,
        ),
        (
            &format!("{FIXTURE_PREFIX} {ACTIVE_STATE} {FIXTURE_FIELDS_BEFORE_START}"),
            LINUX_START_TIME_MISSING_DETAIL,
        ),
        (
            &fixture_stat(ACTIVE_STATE, INVALID_START_TIME),
            LINUX_START_TIME_INVALID_DETAIL,
        ),
    ];
    for (stat, detail) in cases {
        assert!(
            parse_linux_process_stat(stat)
                .unwrap_err()
                .to_string()
                .contains(detail),
            "expected {detail} for {stat}"
        );
    }
}

#[test]
fn zombie_keeps_its_durable_locator_but_is_not_live() {
    let mut child = spawn_immediate_exit();
    let pid = child.id();
    let start_time = process_start_time(pid).unwrap();
    wait_for_state(pid, ZOMBIE_STATE.chars().next().unwrap());

    assert_eq!(process_start_time(pid).unwrap(), start_time);
    assert_eq!(
        try_process_identity_matches(pid, start_time).unwrap(),
        false
    );
    assert!(process_identity_is_inactive(pid, start_time).unwrap());
    assert!(!process_identity_matches(pid, start_time));
    assert!(!is_process_alive(pid));

    child.wait().unwrap();
}

fn fixture_stat(state: &str, start_time: &str) -> String {
    format!("{FIXTURE_PREFIX} {state} {FIXTURE_FIELDS_BEFORE_START} {start_time}")
}

fn spawn_immediate_exit() -> Child {
    crate::process::spawn_waitable_test_child(
        Command::new(SHELL_EXECUTABLE).args([SHELL_COMMAND_ARGUMENT, QUICK_EXIT_SCRIPT]),
    )
}

fn wait_for_state(pid: u32, expected: char) {
    let deadline = Instant::now() + STATE_WAIT_TIMEOUT;
    loop {
        if linux_process_stat(pid).is_ok_and(|stat| stat.state == expected) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "process did not enter {expected}"
        );
        thread::sleep(POLL_INTERVAL);
    }
}

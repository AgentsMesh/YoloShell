use super::*;
use std::process::{Command, Stdio};

const CAT_EXECUTABLE: &str = "/bin/cat";
const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_SECONDS: &str = "30";
const SETTLEMENT_WINDOW: Duration = Duration::from_secs(2);

#[test]
fn failed_watchdog_settlement_reaps_a_child_after_input_eof() {
    let mut child = Command::new(CAT_EXECUTABLE)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    let input = child.stdin.take();
    settle_failed_until(
        child,
        input,
        None,
        Instant::now() + SETTLEMENT_WINDOW,
        SETTLEMENT_WINDOW,
    );
}

#[test]
fn failed_watchdog_settlement_forces_the_retained_exact_child() {
    let child = Command::new(SLEEP_EXECUTABLE)
        .arg(SLEEP_SECONDS)
        .spawn()
        .unwrap();
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    settle_failed_until(
        child,
        None,
        Some(identity),
        Instant::now(),
        SETTLEMENT_WINDOW,
    );
    assert!(!identity.matches());
}

#[test]
fn failed_watchdog_settlement_uses_direct_child_custody_without_identity() {
    let child = Command::new(SLEEP_EXECUTABLE)
        .arg(SLEEP_SECONDS)
        .spawn()
        .unwrap();
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    settle_failed_until(child, None, None, Instant::now(), SETTLEMENT_WINDOW);
    assert!(!identity.matches());
}

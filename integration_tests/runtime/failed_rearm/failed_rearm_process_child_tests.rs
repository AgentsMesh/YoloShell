use super::*;
use std::sync::{Arc, Mutex};
use yolo_runtime::process::ProcessIdentity;

#[test]
fn output_larger_than_pipe_capacity_is_drained_without_deadlock() {
    let mut command = fixture_command(DRAIN_STRESS_MODE).unwrap();
    command
        .arg(FIXTURE_HOME_DIRECTORY)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let completed = custody::spawn(&mut command).unwrap().wait().unwrap();
    assert!(completed.status.success());
    assert_eq!(completed.stdout.len(), DRAIN_STRESS_BYTES);
    assert!(completed.stderr.is_empty());
}

#[test]
fn timeout_force_settles_and_reaps_the_exact_child() {
    let mut command = fixture_command(CAPTURE_STRESS_MODE).unwrap();
    command
        .arg(FIXTURE_HOME_DIRECTORY)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = custody::spawn(&mut command).unwrap();
    let identity = custody::identity_for_test(&child);
    let result = custody::wait_for_test(child, ADVERSARIAL_CHILD_TIMEOUT);
    let Some(error) = result.err() else {
        panic!("{FIXTURE_CHILD_TIMEOUT}");
    };
    assert!(error.to_string().contains(FIXTURE_CHILD_TIMEOUT));
    assert!(!identity.matches());
}

#[test]
fn capture_failure_force_settles_and_reaps_the_uncaptured_child() {
    let mut command = fixture_command(CAPTURE_STRESS_MODE).unwrap();
    command
        .arg(FIXTURE_HOME_DIRECTORY)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let observed = Arc::new(Mutex::new(None));
    let observed_by_capture = Arc::clone(&observed);
    let result = custody::spawn_with_capture_for_test(&mut command, move |pid| {
        *observed_by_capture.lock().unwrap() = Some(ProcessIdentity::capture(pid)?);
        anyhow::bail!(FIXTURE_CAPTURE_FAILURE)
    });
    let Some(error) = result.err() else {
        panic!("{FIXTURE_CAPTURE_FAILURE}");
    };
    assert!(error.to_string().contains(FIXTURE_CAPTURE_FAILURE));
    let Some(identity) = observed.lock().unwrap().take() else {
        panic!("{FIXTURE_CAPTURE_FAILURE}");
    };
    assert!(!identity.matches());
}

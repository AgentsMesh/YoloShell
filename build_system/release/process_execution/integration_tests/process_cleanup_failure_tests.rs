use super::child_lifecycle::{SLEEP_ARGUMENT, SLEEP_EXECUTABLE, assert_reaped, bare_process};
use crate::constants::NULL_DEVICE;
use crate::posix_cleanup::terminate_process;
use crate::posix_spawn::SpawnedProcess;
use crate::posix_supervisor::Streams;
use std::fs::File;

const MISSING_CUSTODY_DETAIL: &str = "spawned process discovery custody is missing";
const DRAIN_DETAIL: &str = "drain child output";
const INPUT_DETAIL: &str = "capture spawned process stdin";
const INPUT_BYTES: &[u8] = b"cleanup-stdin-fixture";

#[test]
fn cleanup_reports_missing_custody_but_still_reaps_the_direct_child() {
    let mut spawned = bare_process(SLEEP_EXECUTABLE, &[SLEEP_ARGUMENT]);
    assert!(spawned.refresh_custody().is_err());
    let pid = spawned.pid;
    let mut streams = Streams::default();
    let error = format!(
        "{:#}",
        terminate_process(&mut spawned, &mut streams).unwrap_err()
    );
    assert!(error.contains(MISSING_CUSTODY_DETAIL), "{error}");
    assert!(streams.complete());
    drop(spawned);
    assert_reaped(pid);
}

#[test]
fn output_read_failure_does_not_skip_process_cleanup_or_hide_custody_failure() {
    let mut spawned = bare_process(SLEEP_EXECUTABLE, &[SLEEP_ARGUMENT]);
    spawned.stdout = File::options().write(true).open(NULL_DEVICE).unwrap();
    let pid = spawned.pid;
    let error = format!(
        "{:#}",
        terminate_process(&mut spawned, &mut Streams::default()).unwrap_err()
    );
    assert!(error.contains(MISSING_CUSTODY_DETAIL), "{error}");
    assert!(error.contains(DRAIN_DETAIL), "{error}");
    drop(spawned);
    assert_reaped(pid);
}

#[test]
fn absent_stdin_is_rejected_without_losing_the_child_cleanup_obligation() {
    let mut spawned = bare_process(SLEEP_EXECUTABLE, &[SLEEP_ARGUMENT]);
    let pid = spawned.pid;
    assert_eq!(
        spawned
            .write_input(Some(INPUT_BYTES))
            .unwrap_err()
            .to_string(),
        INPUT_DETAIL
    );
    spawned.write_input(None).unwrap();
    drop(spawned);
    assert_reaped(pid);
}

#[test]
fn wait_errors_for_a_nonchild_are_observed_without_signalling_it() {
    let output = File::open(NULL_DEVICE).unwrap();
    let mut observed = SpawnedProcess::new(
        std::process::id().try_into().unwrap(),
        output.try_clone().unwrap(),
        output,
        None,
    );
    observed.settle();
    assert!(observed.exited().is_err());
    assert!(observed.wait().is_err());
}

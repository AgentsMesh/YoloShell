use super::*;
use crate::process::{DetachedStandardError, LiveProcessIdentity, ProcessIdentity, spawn_detached};
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use super::monitor_test_support::wait_for;

const ROOT_EXECUTABLE: &str = "/bin/sleep";
const ROOT_PARK_DURATION: &str = "30";
const SHELL_EXECUTABLE: &str = "/bin/sh";
const NON_HOLDER_SCRIPT: &str = "read trigger; sleep 0.1 & wait";
const HOLDER_AFTER_ROOT_SCRIPT: &str = "read trigger; sleep 1 & exit 0";
const ROOT_ONLY_SCRIPT: &str = "read trigger; exit 0";
const ROOT_RELEASE_TRIGGER: &[u8] = b"\n";
const BACKLOGGED_EOF_POISON_DETAIL: &str = "backlogged EOF was not poisoned";

#[test]
fn writer_close_while_root_is_live_stays_poisoned_after_root_exit() {
    let mut child = spawn_detached(
        Path::new(ROOT_EXECUTABLE),
        &[ROOT_PARK_DURATION.into()],
        DetachedStandardError::Null,
    )
    .unwrap();
    let (reader, writer) = monitored_reader(child.id());
    drop(writer);
    wait_for(
        &reader,
        MacosProcessCapabilityObservation::ReleasedPendingRootExit,
    );
    assert_eq!(
        reader.poll_release().unwrap(),
        MacosProcessCapabilityRelease::WriterPresent
    );
    wait_for(
        &reader,
        MacosProcessCapabilityObservation::PrematurelyReleased,
    );
    assert_eq!(
        reader.liveness_observation(),
        Some(MacosProcessCapabilityObservation::PrematurelyReleased)
    );
    assert_poll_poisoned(&reader, release::PREMATURE_RELEASE_DETAIL);
    settle_parked_root(&mut child);
    assert_poll_poisoned(&reader, release::PREMATURE_RELEASE_DETAIL);
}

#[test]
fn non_holder_descendant_after_writer_close_cannot_cure_poison() {
    let mut command = Command::new(SHELL_EXECUTABLE);
    command.arg("-c").arg(NON_HOLDER_SCRIPT);
    let mut child = crate::process::spawn_detached_with_control_pipes(command).unwrap();
    let (reader, writer) = monitored_reader(child.id());
    drop(writer);
    wait_for(
        &reader,
        MacosProcessCapabilityObservation::PrematurelyReleased,
    );
    child
        .stdin
        .take()
        .unwrap()
        .write_all(ROOT_RELEASE_TRIGGER)
        .unwrap();
    child.wait().unwrap();
    assert_eq!(
        reader.liveness_observation(),
        Some(MacosProcessCapabilityObservation::PrematurelyReleased)
    );
}

#[test]
fn inherited_descendant_writer_blocks_eof_until_the_descendant_exits() {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = test_support::transfer(issuer).unwrap();
    let mut command = Command::new(SHELL_EXECUTABLE);
    command.arg("-c").arg(HOLDER_AFTER_ROOT_SCRIPT);
    writer.configure_exec_inheritance(&mut command);
    let mut child = crate::process::spawn_detached_with_control_pipes(command).unwrap();
    drop(writer);
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let live = LiveProcessIdentity::capture(root).unwrap().unwrap();
    reader.arm_liveness(live).unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(ROOT_RELEASE_TRIGGER)
        .unwrap();
    child.wait().unwrap();
    assert_eq!(
        reader.poll_release().unwrap(),
        MacosProcessCapabilityRelease::WriterPresent
    );
    wait_for(
        &reader,
        MacosProcessCapabilityObservation::ReleasedAfterRootExit,
    );
    assert_eq!(
        reader.poll_release().unwrap(),
        MacosProcessCapabilityRelease::Released
    );
}

#[test]
fn root_owned_writer_release_during_kernel_exit_completes() {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = test_support::transfer(issuer).unwrap();
    let mut command = Command::new(SHELL_EXECUTABLE);
    command.arg("-c").arg(ROOT_ONLY_SCRIPT);
    writer.configure_exec_inheritance(&mut command);
    let mut child = crate::process::spawn_detached_with_control_pipes(command).unwrap();
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let live = LiveProcessIdentity::capture(root).unwrap().unwrap();
    reader.verify_writer_holder(&live).unwrap();
    reader.arm_liveness(live).unwrap();
    drop(writer);
    child
        .stdin
        .take()
        .unwrap()
        .write_all(ROOT_RELEASE_TRIGGER)
        .unwrap();
    assert!(child.wait().unwrap().success());
    wait_for(
        &reader,
        MacosProcessCapabilityObservation::ReleasedAfterRootExit,
    );
    assert_eq!(
        reader.poll_release().unwrap(),
        MacosProcessCapabilityRelease::Released
    );
}

#[test]
fn confirmed_exit_racing_backlogged_eof_fails_closed() {
    let mut child = spawn_detached(
        Path::new(ROOT_EXECUTABLE),
        &[ROOT_PARK_DURATION.into()],
        DetachedStandardError::Null,
    )
    .unwrap();
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let live = LiveProcessIdentity::capture(root).unwrap().unwrap();
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = test_support::transfer(issuer).unwrap();
    drop(writer);
    reader.arm_liveness(live).unwrap();
    reader.confirm_root_exit(&live).unwrap();
    assert!(
        matches!(
            reader.liveness_observation(),
            Some(
                MacosProcessCapabilityObservation::PrematurelyReleased
                    | MacosProcessCapabilityObservation::Indeterminate
            )
        ),
        "{BACKLOGGED_EOF_POISON_DETAIL}"
    );
    assert!(reader.poll_release().is_err());
    settle_parked_root(&mut child);
    assert!(reader.poll_release().is_err());
}

fn monitored_reader(
    pid: u32,
) -> (
    Arc<MacosProcessCapabilityManagerReader>,
    MacosProcessCapabilityChildWriter,
) {
    let root = ProcessIdentity::capture(pid).unwrap();
    let live = LiveProcessIdentity::capture(root).unwrap().unwrap();
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = test_support::transfer(issuer).unwrap();
    reader.arm_liveness(live).unwrap();
    (reader, writer)
}

fn settle_parked_root(child: &mut std::process::Child) {
    child.kill().unwrap();
    child.wait().unwrap();
}

fn assert_poll_poisoned(reader: &MacosProcessCapabilityManagerReader, expected: &str) {
    for _attempt in [(), ()] {
        let error = reader.poll_release().unwrap_err();
        assert!(error.to_string().contains(expected));
    }
}

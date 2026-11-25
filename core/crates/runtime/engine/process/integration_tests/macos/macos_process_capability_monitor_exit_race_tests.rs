use super::*;
use crate::process::{LiveProcessIdentity, ProcessIdentity};
use std::io::{Read, Write};
use std::process::Command;

use super::monitor_test_support::wait_for;

const SHELL_EXECUTABLE: &str = "/bin/sh";
const CAPABILITY_DESCRIPTOR_ENVIRONMENT: &str = "YOLO_TEST_PROCESS_CAPABILITY_DESCRIPTOR";
const ROOT_ONLY_SCRIPT: &str =
    "eval \"exec ${YOLO_TEST_PROCESS_CAPABILITY_DESCRIPTOR}>&-\"; printf R; read trigger; exit 0";
const ROOT_READY_MARKER: u8 = b'R';
const ROOT_RELEASE_TRIGGER: &[u8] = b"\n";

#[test]
fn descriptor_release_before_root_exit_event_converges_without_poison() {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = test_support::transfer(issuer).unwrap();
    let mut command = Command::new(SHELL_EXECUTABLE);
    command.arg("-c").arg(ROOT_ONLY_SCRIPT).env(
        CAPABILITY_DESCRIPTOR_ENVIRONMENT,
        writer.descriptor().to_string(),
    );
    writer.configure_exec_inheritance(&mut command);
    let mut child = crate::process::spawn_detached_with_control_pipes(command).unwrap();
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let live = LiveProcessIdentity::capture(root).unwrap().unwrap();
    reader.verify_writer_holder(&live).unwrap();
    reader.arm_liveness(live.clone()).unwrap();
    drop(writer);
    let mut marker = [u8::default(); 1];
    child
        .stdout
        .as_mut()
        .unwrap()
        .read_exact(&mut marker)
        .unwrap();
    assert_eq!(marker, [ROOT_READY_MARKER]);
    wait_for(
        &reader,
        MacosProcessCapabilityObservation::ReleasedPendingRootExit,
    );
    assert_eq!(
        reader.poll_release().unwrap(),
        MacosProcessCapabilityRelease::WriterPresent
    );
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
    reader.confirm_root_exit(&live).unwrap();
    assert_eq!(
        reader.poll_release().unwrap(),
        MacosProcessCapabilityRelease::Released
    );
}

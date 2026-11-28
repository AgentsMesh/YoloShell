use super::*;
use crate::process::{MacosProcessCapabilityIssuer, MacosProcessCapabilityRelease};
use crate::process::{ProcessIdentity, PtySessionProof};
use crate::worker::pty_fault_reaper::FaultReapCompletion;
use crate::worker::pty_ownership::{FaultSettlement, PtyOwnership, ReapEvidence};
use std::process::Command;
use std::time::{Duration, Instant};

const CAPABILITY: &str = "owner-deferred-process-capability";
const CHILD_DURATION: &str = "30";
const CHILD_EXECUTABLE: &str = "/bin/sleep";
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(10);
const SECOND_COMPLETION_TIMEOUT: Duration = Duration::from_millis(25);

#[test]
fn deadline_defers_complete_capability_custody_until_late_eof() {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = issuer.transfer_for_test().unwrap();
    let mut command = Command::new(CHILD_EXECUTABLE);
    command.arg(CHILD_DURATION);
    writer.configure_exec_inheritance_for_test(&mut command);
    let child = crate::process::spawn_waitable_test_child(command);
    drop(writer);
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let mut proof = PtySessionProof::discovery_for_test(CAPABILITY.to_owned()).unwrap();
    proof
        .install_process_capability_for_test(reader.clone())
        .unwrap();
    let mut tree = crate::process::CapturedProcessTree::capture_pty_for_test(root, proof).unwrap();
    tree.authorize_destructive_for_test();
    let reaper = crate::worker::pty_ownership::reserve_fault_reaper().unwrap();
    let mut ownership = PtyOwnership::new_for_test(Box::new(child), tree, reaper).unwrap();
    let mut output = ownership.output_recovery();
    let deadline = CapabilitySettlementDeadline::immediate(Instant::now());

    let cleanup = force_descendants(&mut ownership, &mut output, deadline).unwrap();

    assert!(matches!(cleanup, RootExitCleanup::Deferred(_)));
    assert_eq!(
        reader.poll_release_for_test().unwrap(),
        MacosProcessCapabilityRelease::WriterPresent
    );
    let (completion, receiver) = FaultReapCompletion::owner_channel();
    assert!(matches!(
        ownership.settle_after_fault(completion),
        FaultSettlement::Deferred(_)
    ));
    let outcome = receiver.receive_timeout(COMPLETION_TIMEOUT).unwrap();
    assert_eq!(outcome.evidence, ReapEvidence::Waited);
    assert!(outcome.status.is_some());
    assert_eq!(
        reader.poll_release_for_test().unwrap(),
        MacosProcessCapabilityRelease::Released
    );
    assert!(matches!(
        receiver.receive_timeout(SECOND_COMPLETION_TIMEOUT),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout
            | std::sync::mpsc::RecvTimeoutError::Disconnected)
    ));
}

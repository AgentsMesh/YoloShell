use super::super::super::macos_scan::process_test_support::ReadyProcess;
use super::*;
use crate::process::macos_process_capability::test_support;
use crate::process::{LiveProcessIdentity, MacosProcessCapabilityIssuer, ProcessIdentity};
use std::path::PathBuf;
use std::process::Stdio;
use std::process::{Child, Command};

const TEST_CAPABILITY: &str = "released-candidate-evidence-test";
const OPAQUE_EXECUTABLE: &str = "/bin/sleep";
const OPAQUE_ARGUMENT: &str = "5";
const READABLE_TTY: &str = "/dev/null";

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn retained_discovery_mode_does_not_query_ambient_candidates() {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, _writer) = test_support::transfer(issuer).unwrap();
    let mut proof = PtySessionProof::discovery(TEST_CAPABILITY.to_owned()).unwrap();
    proof.install_process_capability(reader).unwrap();
    let durable = ProcessIdentity::capture(std::process::id()).unwrap();
    let live = LiveProcessIdentity::capture(durable).unwrap().unwrap();
    let candidate = ReleasedCandidate {
        live,
        durable,
        parent_lineage: u64::default(),
        stable: true,
        capability_root: false,
        environment_eligible: false,
    };

    for context in [
        GlobalCandidateContext::ReleasedTty,
        GlobalCandidateContext::Escaped,
    ] {
        let mut released_budget = CapabilityScanBudget::start_batch();
        assert_eq!(
            evaluate_test_candidate(
                &proof,
                candidate,
                &mut released_budget,
                context,
                MacosGlobalScanMode::ReleasedDiscoveryRetained,
            )
            .unwrap(),
            CandidateEvaluation::Complete(false)
        );
        assert_eq!(released_budget.environment_bytes(), usize::default());
        assert_eq!(released_budget.descriptor_count(), usize::default());
    }
}

#[test]
fn released_pty_mode_matches_exact_tty_without_environment_authority() {
    let child = ReadyProcess::spawn(TEST_CAPABILITY, Stdio::null());
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, _writer) = test_support::transfer(issuer).unwrap();
    let mut proof =
        PtySessionProof::capture(PathBuf::from(READABLE_TTY), TEST_CAPABILITY.to_owned()).unwrap();
    proof.install_process_capability(reader).unwrap();
    let durable = ProcessIdentity::capture(child.id()).unwrap();
    let live = LiveProcessIdentity::capture(durable).unwrap().unwrap();
    let candidate = ReleasedCandidate {
        live,
        durable,
        parent_lineage: u64::default(),
        stable: true,
        capability_root: false,
        environment_eligible: true,
    };
    let mut budget = CapabilityScanBudget::start_batch();

    assert_eq!(
        evaluate_test_candidate(
            &proof,
            candidate,
            &mut budget,
            GlobalCandidateContext::ReleasedTty,
            MacosGlobalScanMode::TransferableWriterReleased,
        )
        .unwrap(),
        CandidateEvaluation::Complete(true)
    );
    assert_eq!(budget.environment_bytes(), usize::default());
    assert!(budget.descriptor_count() > usize::default());
}

#[test]
fn opaque_ambient_process_cannot_block_released_discovery() {
    let mut command = Command::new(OPAQUE_EXECUTABLE);
    command.arg(OPAQUE_ARGUMENT);
    let child = ChildGuard(crate::process::spawn_waitable_test_child(command));
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, _writer) = test_support::transfer(issuer).unwrap();
    let mut proof = PtySessionProof::discovery(TEST_CAPABILITY.to_owned()).unwrap();
    proof.install_process_capability(reader).unwrap();
    let durable = ProcessIdentity::capture(child.0.id()).unwrap();
    let live = LiveProcessIdentity::capture(durable).unwrap().unwrap();
    let candidate = ReleasedCandidate {
        live,
        durable,
        parent_lineage: u64::default(),
        stable: true,
        capability_root: false,
        environment_eligible: false,
    };

    let mut present_budget = CapabilityScanBudget::start_batch();
    assert_eq!(
        evaluate_test_candidate(
            &proof,
            candidate,
            &mut present_budget,
            GlobalCandidateContext::Escaped,
            MacosGlobalScanMode::TransferableWriterPresent,
        )
        .unwrap(),
        CandidateEvaluation::Complete(false)
    );
    assert!(present_budget.descriptor_count() > usize::default());
    assert_eq!(present_budget.environment_bytes(), usize::default());

    let mut released_budget = CapabilityScanBudget::start_batch();
    assert_eq!(
        evaluate_test_candidate(
            &proof,
            candidate,
            &mut released_budget,
            GlobalCandidateContext::Escaped,
            MacosGlobalScanMode::ReleasedDiscoveryRetained,
        )
        .unwrap(),
        CandidateEvaluation::Complete(false)
    );
    assert_eq!(released_budget.descriptor_count(), usize::default());
    assert_eq!(released_budget.environment_bytes(), usize::default());
}

#[test]
fn lineage_environment_keeps_an_opaque_process_deferred() {
    let mut command = Command::new(OPAQUE_EXECUTABLE);
    command.arg(OPAQUE_ARGUMENT);
    let child = ChildGuard(crate::process::spawn_waitable_test_child(command));
    let proof = PtySessionProof::discovery(TEST_CAPABILITY.to_owned()).unwrap();
    let durable = ProcessIdentity::capture(child.0.id()).unwrap();
    let live = LiveProcessIdentity::capture(durable).unwrap().unwrap();
    let candidate = ReleasedCandidate {
        live,
        durable,
        parent_lineage: u64::default(),
        stable: true,
        capability_root: false,
        environment_eligible: true,
    };
    let mut budget = CapabilityScanBudget::start_batch();

    assert_eq!(
        evaluate_global_candidate(
            &proof,
            candidate,
            &mut budget,
            GlobalCandidateContext::Escaped,
            MacosGlobalScanMode::LineageEnvironment,
        )
        .unwrap(),
        CandidateEvaluation::Deferred
    );
    assert_eq!(budget.descriptor_count(), usize::default());
}

#[path = "exact/pty_session_released_exact_writer_candidate_tests.rs"]
mod exact_writer_tests;

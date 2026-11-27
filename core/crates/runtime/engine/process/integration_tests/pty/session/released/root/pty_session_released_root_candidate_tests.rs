use super::super::super::macos_candidate::exclude_root;
use super::super::super::macos_scan::process_test_support::ReadyProcess;
use super::*;
use crate::process::macos_process_capability::test_support;
use crate::process::{LiveProcessIdentity, MacosProcessCapabilityIssuer, ProcessIdentity};
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Stdio;

const ROOT_CANDIDATE_CAPABILITY: &str = "released-root-candidate";
const ROOT_CANDIDATE_TTY: &str = "/dev/null";
const UNRELATED_ROOT_CAPABILITY: &str = "unrelated-root-capability";
const ROOT_CANDIDATE_PID: u32 = 41_001;
const ROOT_CANDIDATE_GENERATION: u64 = 51_001;
const REPLACEMENT_GENERATION: u64 = 51_002;

#[test]
fn premature_root_loss_remains_deferred_without_an_explicit_exit_proof() {
    let child = ReadyProcess::spawn(ROOT_CANDIDATE_CAPABILITY, Stdio::null());
    let durable = ProcessIdentity::capture(child.id()).unwrap();
    let live = LiveProcessIdentity::capture(durable).unwrap().unwrap();
    let candidate = ReleasedCandidate {
        live,
        durable,
        parent_lineage: u64::default(),
        stable: true,
        capability_root: true,
        environment_eligible: true,
    };
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, _writer) = test_support::transfer(issuer).unwrap();
    let mut proof = PtySessionProof::capture(
        PathBuf::from(ROOT_CANDIDATE_TTY),
        ROOT_CANDIDATE_CAPABILITY.to_owned(),
    )
    .unwrap();
    proof.install_process_capability(reader).unwrap();
    assert!(candidate.live.try_matches().unwrap());
    assert!(candidate.live.signal(libc::SIGKILL).unwrap());
    drop(child);

    let mut budget = CapabilityScanBudget::start_batch();
    assert_eq!(
        evaluate_global_candidate(
            &proof,
            candidate,
            &mut budget,
            GlobalCandidateContext::Escaped,
            MacosGlobalScanMode::PrematureBootstrapRelease,
        )
        .unwrap(),
        CandidateEvaluation::Deferred
    );
}

#[test]
fn exited_root_exclusion_keeps_a_same_pid_replacement_outside_the_exclusion() {
    let root =
        LiveProcessIdentity::from_kernel(ROOT_CANDIDATE_PID, ROOT_CANDIDATE_GENERATION).unwrap();
    let replacement =
        LiveProcessIdentity::from_kernel(ROOT_CANDIDATE_PID, REPLACEMENT_GENERATION).unwrap();
    let priorities = HashSet::from([root, replacement]);

    let excluded = exclude_root(&HashSet::new(), &priorities, root.lineage_id(), true);

    assert_eq!(excluded, HashSet::from([root]));
    assert!(!excluded.contains(&replacement));
    assert!(!exclude_root(&HashSet::new(), &priorities, root.lineage_id(), false).contains(&root));
}

#[test]
fn live_root_candidate_still_requires_the_normal_descriptor_fence() {
    let child = ReadyProcess::spawn(ROOT_CANDIDATE_CAPABILITY, Stdio::null());
    let durable = ProcessIdentity::capture(child.id()).unwrap();
    let live = LiveProcessIdentity::capture(durable).unwrap().unwrap();
    let candidate = ReleasedCandidate {
        live,
        durable,
        parent_lineage: u64::default(),
        stable: true,
        capability_root: true,
        environment_eligible: true,
    };
    let proof = PtySessionProof::discovery(UNRELATED_ROOT_CAPABILITY.to_owned()).unwrap();
    let mut budget = CapabilityScanBudget::start_batch();
    assert_eq!(
        evaluate_global_candidate(
            &proof,
            candidate,
            &mut budget,
            GlobalCandidateContext::Escaped,
            MacosGlobalScanMode::PrematureBootstrapRelease,
        )
        .unwrap(),
        CandidateEvaluation::Complete(false)
    );
}

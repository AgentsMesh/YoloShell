use super::super::super::macos_scan::process_test_support::ReadyProcess;
use super::super::{ReleasedScanState, advance_scan};
use super::*;
use crate::process::macos_process_capability::test_support;
use crate::process::{
    LiveProcessIdentity, MacosProcessCapabilityIssuer, ProcessIdentity, PtySessionProof,
};
use std::cell::Cell;
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

const EXTERNAL_ENVIRONMENT_CAPABILITY: &str = "external-environment-cannot-authorize";
const READABLE_TTY: &str = "/dev/null";
const SINGLE_EXTERNAL_CANDIDATE: usize = 1;
const STALE_GENERATION_INCREMENT: u64 = 1;

#[test]
fn external_exact_tty_holder_is_owned_after_writer_release() {
    let child = ReadyProcess::spawn(EXTERNAL_ENVIRONMENT_CAPABILITY, Stdio::piped());
    let (proof, _writer) = proof_with_writer();
    let candidate = external_candidate(ProcessIdentity::capture(child.id()).unwrap());

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
    assert_eq!(present_budget.environment_bytes(), usize::default());
    assert!(present_budget.descriptor_count() > usize::default());

    let mut released_budget = CapabilityScanBudget::start_batch();
    assert_eq!(
        evaluate_test_candidate(
            &proof,
            candidate,
            &mut released_budget,
            GlobalCandidateContext::Escaped,
            MacosGlobalScanMode::TransferableWriterReleased,
        )
        .unwrap(),
        CandidateEvaluation::Complete(true)
    );
    assert_eq!(released_budget.environment_bytes(), usize::default());
    assert!(released_budget.descriptor_count() > usize::default());
}

#[test]
fn released_external_epoch_scans_tty_and_retains_exact_generation_fence() {
    let child = ReadyProcess::spawn(EXTERNAL_ENVIRONMENT_CAPABILITY, Stdio::piped());
    let (proof, _writer) = proof_with_writer();
    let candidate = external_candidate(ProcessIdentity::capture(child.id()).unwrap());
    let query_work = Cell::new(None);
    let scan = advance_scan(
        Arc::from([candidate]),
        &ReleasedScanState::default(),
        SINGLE_EXTERNAL_CANDIDATE,
        |candidate, budget| {
            let result = evaluate_test_candidate(
                &proof,
                candidate,
                budget,
                GlobalCandidateContext::Escaped,
                MacosGlobalScanMode::TransferableWriterReleased,
            );
            query_work.set(Some((
                budget.environment_bytes(),
                budget.descriptor_count(),
            )));
            result
        },
    )
    .unwrap();
    assert_eq!(
        scan.candidates.status,
        super::super::CapabilityScanStatus::Pending
    );
    assert_eq!(
        scan.candidates.observed,
        HashSet::from([candidate.live.lineage_id()])
    );
    let (environment, descriptors) = query_work.get().unwrap();
    assert_eq!(environment, usize::default());
    assert!(descriptors > usize::default());

    let stale = ReleasedCandidate {
        live: candidate.live.with_generation(
            candidate
                .live
                .unique_id()
                .wrapping_add(STALE_GENERATION_INCREMENT),
        ),
        ..candidate
    };
    let stale_scan = advance_scan(
        Arc::from([stale]),
        &ReleasedScanState::default(),
        SINGLE_EXTERNAL_CANDIDATE,
        |candidate, budget| {
            evaluate_test_candidate(
                &proof,
                candidate,
                budget,
                GlobalCandidateContext::Escaped,
                MacosGlobalScanMode::TransferableWriterReleased,
            )
        },
    )
    .unwrap();
    assert_eq!(
        stale_scan.candidates.status,
        super::super::CapabilityScanStatus::Pending
    );
}

fn proof_with_writer() -> (
    PtySessionProof,
    crate::process::MacosProcessCapabilityChildWriter,
) {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = test_support::transfer(issuer).unwrap();
    let mut proof = PtySessionProof::capture(
        PathBuf::from(READABLE_TTY),
        EXTERNAL_ENVIRONMENT_CAPABILITY.to_owned(),
    )
    .unwrap();
    proof.install_process_capability(reader).unwrap();
    (proof, writer)
}

fn external_candidate(identity: ProcessIdentity) -> ReleasedCandidate {
    ReleasedCandidate {
        live: LiveProcessIdentity::capture(identity).unwrap().unwrap(),
        durable: identity,
        parent_lineage: u64::default(),
        stable: true,
        capability_root: false,
        environment_eligible: false,
    }
}

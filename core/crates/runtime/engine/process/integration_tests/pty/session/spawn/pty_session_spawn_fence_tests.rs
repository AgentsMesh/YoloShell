use super::*;
use crate::process::MacosProcessCapabilityIssuer;
use portable_pty::{PtySize, native_pty_system};

const TEST_CAPABILITY: &str = "descriptor-spawn-fence-test";
const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;

#[test]
fn writer_present_candidate_defers_before_querying_its_environment() {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (authority, _writer) = issuer.transfer_for_test().unwrap();
    let mut proof = PtySessionProof::discovery(TEST_CAPABILITY.to_owned()).unwrap();
    proof.install_process_capability(authority).unwrap();
    let mut budget = CapabilityScanBudget::start(SINGLE_TEST_CANDIDATE).unwrap();
    let _spawn = crate::process::waitable_child_spawn_guard().unwrap();

    let error = proof
        .matches_escaped_process(std::process::id(), &mut budget)
        .unwrap_err();

    assert!(matches!(
        error,
        CapabilityProofError::DescriptorScanDeferred(_)
    ));
    assert_eq!(budget.environment_bytes(), usize::default());
    assert_eq!(budget.descriptor_count(), usize::default());
}

#[test]
fn available_candidate_defers_before_querying_tty_or_environment() {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: TEST_ROWS,
            cols: TEST_COLUMNS,
            pixel_width: TEST_PIXEL_SIZE,
            pixel_height: TEST_PIXEL_SIZE,
        })
        .unwrap();
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (authority, _writer) = issuer.transfer_for_test().unwrap();
    let mut proof =
        PtySessionProof::capture(pair.master.tty_name().unwrap(), TEST_CAPABILITY.to_owned())
            .unwrap();
    proof.install_process_capability(authority).unwrap();
    let mut budget = CapabilityScanBudget::start(SINGLE_TEST_CANDIDATE).unwrap();
    let _spawn = crate::process::waitable_child_spawn_guard().unwrap();

    let error = proof
        .matches_available_process(std::process::id(), &mut budget)
        .unwrap_err();

    assert!(matches!(
        error,
        CapabilityProofError::DescriptorScanDeferred(_)
    ));
    assert_eq!(budget.environment_bytes(), usize::default());
    assert_eq!(budget.descriptor_count(), usize::default());
}

#[test]
fn environment_only_candidate_does_not_need_the_descriptor_spawn_gate() {
    let proof = PtySessionProof::discovery(TEST_CAPABILITY.to_owned()).unwrap();
    let mut budget = CapabilityScanBudget::start(SINGLE_TEST_CANDIDATE).unwrap();
    let _spawn = crate::process::waitable_child_spawn_guard().unwrap();

    let outcome = fenced_candidate(
        || Ok(true),
        || proof.matches_escaped_process(std::process::id(), &mut budget),
    )
    .unwrap();

    assert_eq!(outcome, CandidateEvaluation::Complete(false));
    assert!(budget.environment_bytes() > usize::default());
    assert_eq!(budget.descriptor_count(), usize::default());
}

#[test]
fn raw_descriptor_proof_remains_callable_inside_spawn_commit() {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (authority, _writer) = issuer.transfer_for_test().unwrap();
    let mut budget = CapabilityScanBudget::start(SINGLE_TEST_CANDIDATE).unwrap();
    let _spawn = crate::process::waitable_child_spawn_guard().unwrap();

    let matched = super::super::macos_descriptor_capability::has_stable_descriptor_capability(
        std::process::id(),
        authority.writer_handle(),
        &mut budget,
    )
    .unwrap();

    assert!(matched);
}

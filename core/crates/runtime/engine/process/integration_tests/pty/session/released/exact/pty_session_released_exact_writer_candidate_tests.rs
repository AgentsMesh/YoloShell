use super::*;

#[test]
fn matching_environment_without_exact_writer_is_not_owned() {
    let child = ReadyProcess::spawn(TEST_CAPABILITY, Stdio::piped());
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, _writer) = test_support::transfer(issuer).unwrap();
    let mut proof = PtySessionProof::discovery(TEST_CAPABILITY.to_owned()).unwrap();
    proof.install_process_capability(reader).unwrap();
    let durable = ProcessIdentity::capture(child.id()).unwrap();
    let candidate = ReleasedCandidate {
        live: LiveProcessIdentity::capture(durable).unwrap().unwrap(),
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
            GlobalCandidateContext::Escaped,
            MacosGlobalScanMode::TransferableWriterPresent,
        )
        .unwrap(),
        CandidateEvaluation::Complete(false)
    );
    assert_eq!(budget.environment_bytes(), usize::default());
    assert!(budget.descriptor_count() > usize::default());
}

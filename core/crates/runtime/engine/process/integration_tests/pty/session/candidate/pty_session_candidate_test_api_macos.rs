use super::*;

const TEST_CAPABILITY: &str = "candidate-test-api";

#[test]
fn process_match_reports_spawn_gate_deferral() {
    let issuer = crate::process::MacosProcessCapabilityIssuer::create().unwrap();
    let (authority, _writer) = issuer.transfer_for_test().unwrap();
    let mut proof = PtySessionProof::discovery(TEST_CAPABILITY.to_owned()).unwrap();
    proof.install_process_capability(authority).unwrap();
    let _spawn = crate::process::waitable_child_spawn_guard().unwrap();

    assert_eq!(
        proof.matches_process_for_test(std::process::id()).unwrap(),
        None
    );
}

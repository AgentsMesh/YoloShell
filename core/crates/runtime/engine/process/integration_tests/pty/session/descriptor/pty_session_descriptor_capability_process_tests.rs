use super::*;
use crate::process::MacosProcessCapabilityIssuer;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

const PROTECTED_EXECUTABLE: &str = "/bin/sleep";
const PROTECTED_ARGUMENT: &str = "5";
const TEST_CAPABILITY: &str = "descriptor-capability-process-test";
const DESCRIPTOR_SCAN_SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(2);
const DESCRIPTOR_SCAN_SETTLEMENT_FAILURE: &str =
    "descriptor scan gate stayed busy while testing capability absence";

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn inherited_pipe_handle_matches_the_current_process() {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (authority, _writer) = issuer.transfer_for_test().unwrap();
    let mut proof = PtySessionProof::discovery(TEST_CAPABILITY.to_owned()).unwrap();
    proof.install_process_capability(authority).unwrap();
    let mut budget = CapabilityScanBudget::start(SINGLE_TEST_CANDIDATE).unwrap();
    assert!(
        proof
            .matches_escaped_process(std::process::id(), &mut budget)
            .unwrap()
    );
}

#[test]
fn descriptor_absence_excludes_an_opaque_protected_process() {
    let mut command = Command::new(PROTECTED_EXECUTABLE);
    command.arg(PROTECTED_ARGUMENT);
    let mut child = ChildGuard(crate::process::spawn_waitable_test_child(command));
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (authority, _writer) = issuer.transfer_for_test().unwrap();
    let mut proof = PtySessionProof::discovery(TEST_CAPABILITY.to_owned()).unwrap();
    let mut budget = CapabilityScanBudget::start(SINGLE_TEST_CANDIDATE).unwrap();
    assert!(
        proof
            .matches_escaped_process(child.0.id(), &mut budget)
            .is_err()
    );
    proof.install_process_capability(authority).unwrap();
    assert!(!matches_after_spawn_settlement(&proof, child.0.id()));
    child.0.kill().unwrap();
    child.0.wait().unwrap();
}

fn matches_after_spawn_settlement(proof: &PtySessionProof, pid: u32) -> bool {
    let deadline = Instant::now() + DESCRIPTOR_SCAN_SETTLEMENT_TIMEOUT;
    loop {
        let mut budget = CapabilityScanBudget::start(SINGLE_TEST_CANDIDATE).unwrap();
        match proof.matches_escaped_process(pid, &mut budget) {
            Ok(matches) => return matches,
            Err(CapabilityProofError::DescriptorScanDeferred(_)) => {
                assert!(
                    Instant::now() < deadline,
                    "{DESCRIPTOR_SCAN_SETTLEMENT_FAILURE}"
                );
                std::thread::yield_now();
            }
            Err(error) => panic!("{error:#}"),
        }
    }
}

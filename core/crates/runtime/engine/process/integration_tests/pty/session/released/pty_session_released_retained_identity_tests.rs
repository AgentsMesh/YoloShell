use super::*;
use crate::process::MacosProcessCapabilityIssuer;
use crate::process::macos_process_capability::test_support;
use std::process::{Child, Command};

const RETAINED_CAPABILITY: &str = "released-retained-identity";
const OPAQUE_EXECUTABLE: &str = "/bin/sleep";
const OPAQUE_ARGUMENT: &str = "5";

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn released_scan_replays_an_omitted_exact_identity_and_ignores_ambient_processes() {
    let member = ReadyProcess::spawn(RETAINED_CAPABILITY, Stdio::null());
    let member_durable = crate::process::ProcessIdentity::capture(member.id()).unwrap();
    let member_candidate = live_candidate(member_durable);
    let mut state = ReleasedScanState::default();
    let matched = advance_scan(
        candidates([member_candidate]),
        &state,
        MAX_PTY_PROOF_CANDIDATES,
        |_candidate, _budget| Ok(CandidateEvaluation::Complete(true)),
    )
    .unwrap();
    commit(&mut state, matched);
    state = state.restart_preserving_matches();

    let mut command = Command::new(OPAQUE_EXECUTABLE);
    command.arg(OPAQUE_ARGUMENT);
    let opaque = ChildGuard(crate::process::spawn_waitable_test_child(command));
    let opaque_durable = crate::process::ProcessIdentity::capture(opaque.0.id()).unwrap();
    let opaque_entry = crate::process::inventory_test_support::entry(
        opaque_durable,
        u32::default(),
        LIVE_SCAN_SESSION_ID,
    );
    let snapshot = inventory(
        vec![opaque_entry],
        HashSet::from([opaque_entry.lineage_id()]),
    );
    let proof = released_discovery_proof();
    let retained = scan_escaped(
        &snapshot,
        &proof,
        &state,
        member_durable.start_time,
        member_candidate.live.lineage_id(),
        MacosGlobalScanMode::ReleasedDiscoveryRetained,
        false,
        &HashSet::new(),
        &HashSet::new(),
    )
    .unwrap();

    assert_eq!(retained.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(retained.candidates.status, CapabilityScanStatus::Complete);
    assert_eq!(
        retained.candidates.observed,
        HashSet::from([member_candidate.live.lineage_id()])
    );
    assert!(
        !retained
            .candidates
            .observed
            .contains(&opaque_entry.lineage_id())
    );
    commit(&mut state, retained);

    drop(member);
    let pruned = scan_escaped(
        &snapshot,
        &proof,
        &state,
        member_durable.start_time,
        member_candidate.live.lineage_id(),
        MacosGlobalScanMode::ReleasedDiscoveryRetained,
        false,
        &HashSet::new(),
        &HashSet::new(),
    )
    .unwrap();
    assert_eq!(pruned.candidates.status, CapabilityScanStatus::Complete);
    assert!(pruned.candidates.observed.is_empty());
}

fn live_candidate(durable: crate::process::ProcessIdentity) -> ReleasedCandidate {
    ReleasedCandidate {
        live: LiveProcessIdentity::capture(durable).unwrap().unwrap(),
        durable,
        parent_lineage: u64::default(),
        stable: true,
        capability_root: false,
        environment_eligible: false,
    }
}

fn released_discovery_proof() -> PtySessionProof {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = test_support::transfer(issuer).unwrap();
    let mut proof = PtySessionProof::discovery(RETAINED_CAPABILITY.to_owned()).unwrap();
    proof.install_process_capability(reader).unwrap();
    drop(writer);
    proof
}

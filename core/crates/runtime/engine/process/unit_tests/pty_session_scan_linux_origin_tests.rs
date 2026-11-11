use super::*;
use crate::process::inventory_snapshot::install_test_inventory;

const ISSUER: ProcessIdentity = ProcessIdentity {
    pid: 10,
    start_time: 100,
};
const ROOT: ProcessIdentity = ProcessIdentity {
    pid: 20,
    start_time: 200,
};
const CANDIDATE_PID: u32 = 30;
const STEP: u64 = 1;
const CAPABILITY: &str = "closed-discovery-test-capability";

fn candidate(start_time: u64) -> InventoryEntry {
    InventoryEntry {
        pid: CANDIDATE_PID,
        parent_pid: u32::default(),
        process_group: CANDIDATE_PID,
        session_id: CANDIDATE_PID,
        start_time,
        effective_user_id: unsafe { libc::geteuid() },
    }
}
fn inventory(entries: Vec<InventoryEntry>) -> StableInventory {
    let _guard = install_test_inventory([entries.clone(), entries]);
    StableInventory::capture().unwrap()
}
fn proof(scoped: bool) -> PtySessionProof {
    let mut proof = PtySessionProof::discovery(CAPABILITY.into()).unwrap();
    if scoped {
        proof
            .bind_linux_capability_origin(Some(ISSUER), ROOT)
            .unwrap();
    }
    proof
}

#[test]
fn preissuer_opaque_process_is_excluded_before_any_environment_or_descriptor_query() {
    let inventory = inventory(vec![candidate(ISSUER.start_time - STEP)]);
    let scoped = proof(true);
    let result = scan_with(
        &inventory,
        &scoped,
        &LinuxScanState::default(),
        (ROOT, false, &HashSet::new(), &HashSet::new()),
        |_, _| panic!("older unrelated process must not be queried"),
    )
    .unwrap();
    assert_eq!(result.coverage_status, CapabilityScanStatus::Complete);
    assert!(result.candidates.identities.is_empty());
}

#[test]
fn possible_members_keep_permission_uncertainty_including_pre_root_siblings_and_escaped_sessions() {
    for (scoped, birth) in [
        (false, ISSUER.start_time - STEP),
        (true, ISSUER.start_time),
        (true, ROOT.start_time - STEP),
        (true, ROOT.start_time + STEP),
    ] {
        let inventory = inventory(vec![candidate(birth)]);
        let mut queries = 0;
        let result = scan_with(
            &inventory,
            &proof(scoped),
            &LinuxScanState::default(),
            (ROOT, true, &HashSet::new(), &HashSet::new()),
            |_, _| {
                queries += 1;
                EvaluatedCandidate::Pending(CandidatePending::Permission(QueryKind::Environment))
            },
        )
        .unwrap();
        assert_eq!(queries, 1);
        assert_eq!(result.coverage_status, CapabilityScanStatus::Pending);
    }
}

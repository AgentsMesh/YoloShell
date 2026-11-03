use super::*;

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
const CAPABILITY: &str = "origin-bound-test-capability";

fn entry(start_time: u64) -> InventoryEntry {
    InventoryEntry {
        pid: CANDIDATE_PID,
        parent_pid: u32::default(),
        process_group: CANDIDATE_PID,
        session_id: CANDIDATE_PID,
        start_time,
        effective_user_id: unsafe { libc::geteuid() },
    }
}

#[test]
fn only_strictly_preissuer_unrelated_processes_can_be_excluded() {
    let mut proof = PtySessionProof::discovery(CAPABILITY.into()).unwrap();
    proof
        .bind_linux_capability_origin(Some(ISSUER), ROOT)
        .unwrap();
    assert!(!proof.linux_candidate_relevant(entry(ISSUER.start_time - STEP), ROOT, false));
    for birth in [
        u64::default(),
        ISSUER.start_time,
        ROOT.start_time - STEP,
        ROOT.start_time,
        ROOT.start_time + STEP,
    ] {
        assert!(proof.linux_candidate_relevant(entry(birth), ROOT, false));
    }
    let mut older = entry(ISSUER.start_time - STEP);
    assert!(proof.linux_candidate_relevant(older, ROOT, true));
    older.parent_pid = ROOT.pid;
    assert!(proof.linux_candidate_relevant(older, ROOT, false));
    older.parent_pid = u32::default();
    older.session_id = ROOT.pid;
    assert!(proof.linux_candidate_relevant(older, ROOT, false));
    older.session_id = CANDIDATE_PID;
    older.pid = ROOT.pid;
    assert!(proof.linux_candidate_relevant(older, ROOT, false));
}

#[test]
fn missing_or_inconsistent_provenance_never_excludes_and_clone_keeps_issued_boundary() {
    let mut proof = PtySessionProof::discovery(CAPABILITY.into()).unwrap();
    let older = entry(ISSUER.start_time - STEP);
    assert!(proof.linux_candidate_relevant(older, ROOT, false));
    assert!(
        proof
            .bind_linux_capability_origin(
                Some(ProcessIdentity {
                    start_time: ROOT.start_time + STEP,
                    ..ISSUER
                }),
                ROOT
            )
            .is_err()
    );
    assert!(proof.linux_candidate_relevant(older, ROOT, false));
    proof
        .bind_linux_capability_origin(Some(ISSUER), ROOT)
        .unwrap();
    assert!(!proof.clone().linux_candidate_relevant(older, ROOT, false));
}

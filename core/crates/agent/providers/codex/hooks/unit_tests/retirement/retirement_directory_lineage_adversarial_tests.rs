use super::retirement_test_constants::{DISPLACED_INVOCATION_DIRECTORY, SNAPSHOT_LINEAGE_FIELD};
use super::retirement_test_support::RetirementFixture;
use crate::agent::hooks::ReducerSnapshot;
use crate::codex_hooks::{
    CodexAdmissionAuthority, CodexRetirementRecord, CodexRetirementSeal, CodexSnapshotCursor,
    CodexSnapshotProof,
};
use crate::process::{ExactProcessGone, ProcessIdentityObservation};

#[test]
fn target_writer_rejects_foreign_admission_with_the_same_hook_identity() {
    let target = RetirementFixture::new();
    let foreign = RetirementFixture::with_identity(target.owner().clone());
    let writer = target.writer();
    let foreign_seal = foreign.seal();

    foreign_seal.validate_for(foreign.admission()).unwrap();
    assert!(
        writer
            .publish_seal(&foreign_seal, foreign.admission())
            .is_err()
    );
    assert!(!target.seal_path().exists());
}

#[test]
fn foreign_snapshot_lineage_cannot_publish_into_the_target_directory() {
    let target = RetirementFixture::new();
    let foreign = RetirementFixture::with_identity(target.owner().clone());
    let writer = target.writer();
    let seal = writer
        .publish_seal(&target.seal(), target.admission())
        .unwrap();
    let terminal = target.terminal(&writer, seal.artifact());
    let foreign_proof = persist_snapshot(&foreign, &terminal.snapshot);
    let forged = bind_record_to_proof(&terminal.record, &foreign_proof);

    forged
        .validate_for(
            &terminal.snapshot,
            target.admission(),
            seal.artifact(),
            &foreign_proof,
        )
        .unwrap();
    assert!(
        writer
            .publish_retirement(
                &forged,
                seal.artifact(),
                &terminal.snapshot,
                target.admission(),
                &foreign_proof,
                &terminal.intake,
                &terminal.outbox,
            )
            .is_err()
    );
    assert!(!target.retirement_path().exists());
}

#[test]
fn retained_writer_rejects_invocation_directory_path_substitution() {
    let target = RetirementFixture::new();
    let foreign = RetirementFixture::with_identity(target.owner().clone());
    let writer = target.writer();
    let target_directory = target.invocation_directory().to_path_buf();
    let displaced = target_directory.with_file_name(DISPLACED_INVOCATION_DIRECTORY);

    std::fs::rename(&target_directory, displaced).unwrap();
    std::fs::rename(foreign.invocation_directory(), &target_directory).unwrap();
    let substituted_admission = target.recovered_admission();
    let substituted_seal = capture_seal(&substituted_admission);

    substituted_admission.verify().unwrap();
    substituted_seal
        .validate_for(&substituted_admission)
        .unwrap();
    assert!(
        writer
            .publish_seal(&substituted_seal, &substituted_admission)
            .is_err()
    );
    assert!(!target.seal_path().exists());
}

fn bind_record_to_proof(
    record: &CodexRetirementRecord,
    proof: &CodexSnapshotProof,
) -> CodexRetirementRecord {
    let mut encoded = serde_json::to_value(record).unwrap();
    encoded[SNAPSHOT_LINEAGE_FIELD] = serde_json::to_value(proof.lineage().unwrap()).unwrap();
    serde_json::from_value(encoded).unwrap()
}

fn persist_snapshot(fixture: &RetirementFixture, snapshot: &ReducerSnapshot) -> CodexSnapshotProof {
    let (cursor, loaded) = CodexSnapshotCursor::open(fixture.snapshot_path()).unwrap();
    assert!(loaded.is_none());
    cursor.persist(snapshot).unwrap().prove_current().unwrap()
}

fn capture_seal(admission: &CodexAdmissionAuthority) -> CodexRetirementSeal {
    let owner = &admission.record().request().identity;
    let process = crate::process::ProcessIdentity {
        pid: owner.process.pid,
        start_time: owner.process.start_time,
    };
    let gone =
        ExactProcessGone::capture_for_test(process, ProcessIdentityObservation::Gone).unwrap();
    CodexRetirementSeal::capture(admission, gone).unwrap()
}

use super::retirement_test_constants::DISPLACED_INVOCATION_DIRECTORY;
use super::retirement_test_support::{
    MALFORMED_JSON, RetirementFixture, replace_same_content, replace_with,
};
use crate::codex_hooks::CodexRetirementWriterFence;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
enum RetiredMutation {
    AdmissionIdentity,
    SealIdentity,
    SnapshotIdentity,
    SnapshotLockIdentity,
}

#[test]
fn cold_read_rejects_retirement_without_its_seal() {
    let fixture = RetirementFixture::new();
    let writer = install_retired(&fixture);
    std::fs::remove_file(fixture.seal_path()).unwrap();

    assert!(writer.read_state(fixture.admission()).is_err());
}

#[test]
fn cold_read_rejects_a_malformed_seal() {
    let malformed_seal = RetirementFixture::new();
    replace_with(&malformed_seal.seal_path(), MALFORMED_JSON);
    assert!(
        malformed_seal
            .writer()
            .read_state(malformed_seal.admission())
            .is_err()
    );
}

#[test]
fn malformed_retirement_cannot_degrade_to_sealed() {
    let malformed_retirement = RetirementFixture::new();
    let writer = malformed_retirement.writer();
    writer
        .publish_seal(
            &malformed_retirement.seal(),
            malformed_retirement.admission(),
        )
        .unwrap();
    replace_with(&malformed_retirement.retirement_path(), MALFORMED_JSON);
    assert!(writer.read_state(malformed_retirement.admission()).is_err());
}

#[test]
fn cold_read_rejects_same_payload_new_inode_lineage() {
    for mutation in [
        RetiredMutation::AdmissionIdentity,
        RetiredMutation::SealIdentity,
        RetiredMutation::SnapshotIdentity,
        RetiredMutation::SnapshotLockIdentity,
    ] {
        let fixture = RetirementFixture::new();
        let writer = install_retired(&fixture);
        mutate_retired(&fixture, mutation);

        assert!(writer.read_state(fixture.admission()).is_err());
    }
}

#[test]
fn cold_read_requires_the_exact_terminal_snapshot() {
    for contents in [None, Some(MALFORMED_JSON)] {
        let fixture = RetirementFixture::new();
        let writer = install_retired(&fixture);
        match contents {
            Some(contents) => replace_with(&fixture.snapshot_path(), contents),
            None => std::fs::remove_file(fixture.snapshot_path()).unwrap(),
        }

        assert!(writer.read_state(fixture.admission()).is_err());
    }
}

#[test]
fn retained_cold_reader_rejects_invocation_path_substitution() {
    let target = RetirementFixture::new();
    let foreign = RetirementFixture::new();
    let writer = target.writer();
    let target_directory = invocation_directory(&target);
    let displaced = target_directory.with_file_name(DISPLACED_INVOCATION_DIRECTORY);

    std::fs::rename(&target_directory, displaced).unwrap();
    std::fs::rename(invocation_directory(&foreign), &target_directory).unwrap();

    assert!(writer.read_state(target.admission()).is_err());
}

fn install_retired(fixture: &RetirementFixture) -> CodexRetirementWriterFence {
    let writer = fixture.writer();
    let seal = writer
        .publish_seal(&fixture.seal(), fixture.admission())
        .unwrap();
    let terminal = fixture.terminal(&writer, seal.artifact());
    writer
        .publish_retirement(
            &terminal.record,
            seal.artifact(),
            &terminal.snapshot,
            fixture.admission(),
            &terminal.proof,
            &terminal.intake,
            &terminal.outbox,
        )
        .unwrap();
    writer
}

fn mutate_retired(fixture: &RetirementFixture, mutation: RetiredMutation) {
    let path = match mutation {
        RetiredMutation::AdmissionIdentity => fixture.admission_path().to_path_buf(),
        RetiredMutation::SealIdentity => fixture.seal_path(),
        RetiredMutation::SnapshotIdentity => fixture.snapshot_path(),
        RetiredMutation::SnapshotLockIdentity => snapshot_lock_path(fixture),
    };
    replace_same_content(&path);
}

fn snapshot_lock_path(fixture: &RetirementFixture) -> PathBuf {
    let mut path = fixture.snapshot_path().into_os_string();
    path.push(super::super::constants::SNAPSHOT_LOCK_SUFFIX);
    PathBuf::from(path)
}

fn invocation_directory(fixture: &RetirementFixture) -> PathBuf {
    parent(fixture.admission_path()).to_path_buf()
}

fn parent(path: &Path) -> &Path {
    path.parent().unwrap()
}

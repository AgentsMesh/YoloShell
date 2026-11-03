use super::commit_cursor_format::{CursorRecord, decode_cursor, encode_cursor};
use super::tests_retained_hooks::inject_parent_sync_failure;
use super::tests_retained_support::RetainedFixture;
use super::tests_security_support::private_root;
use super::*;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

const CURSOR_LOAD_FILE: &str = "cursor-load-matrix";
const CURSOR_REVISION_STEP: u64 = CURSOR_REVISION_INCREMENT;

#[derive(Clone, Copy)]
enum RestoreMutation {
    Generation,
    BeforeSeal,
    BeyondFile,
}

#[derive(Clone, Copy)]
enum RetiredMutation {
    Generation,
    NewerRevision,
    SameRevisionConflict,
    SameRecord,
}

#[derive(Clone, Copy)]
enum CurrentCommitMutation {
    BeforeSeal,
    BeyondFile,
}

#[test]
fn invalid_isolated_cursor_never_restores_over_a_missing_current_cursor() {
    for mutation in [
        RestoreMutation::Generation,
        RestoreMutation::BeforeSeal,
        RestoreMutation::BeyondFile,
    ] {
        let (fixture, isolated) = isolated_fixture();
        let sealed = fixture.journal.header.sealed_bytes;
        let file_len = fs::metadata(&fixture.moved_journal).unwrap().len();
        let record = CursorRecord {
            generation: match mutation {
                RestoreMutation::Generation => Uuid::new_v4(),
                _ => fixture.attempt.generation,
            },
            committed_bytes: match mutation {
                RestoreMutation::BeforeSeal => sealed - INITIAL_RETAINED_SEQUENCE,
                RestoreMutation::BeyondFile => file_len + INITIAL_RETAINED_SEQUENCE,
                RestoreMutation::Generation => sealed,
            },
            publication_revision: INITIAL_CURSOR_REVISION,
        };
        fs::write(isolated, encode_cursor(record)).unwrap();
        fs::remove_file(&fixture.moved_cursor).unwrap();

        assert!(matches!(
            Journal::open(&fixture.moved_journal, fixture.journal.runtime_epoch()),
            Err(JournalError::InvalidFormat)
        ));
        fixture.assert_replacements_unchanged();
    }
}

#[test]
fn retired_cursor_settlement_accepts_only_nonconflicting_history() {
    for mutation in [
        RetiredMutation::Generation,
        RetiredMutation::NewerRevision,
        RetiredMutation::SameRevisionConflict,
        RetiredMutation::SameRecord,
    ] {
        let (fixture, isolated) = isolated_fixture();
        let current = active_cursor(&fixture.moved_cursor);
        let retired = match mutation {
            RetiredMutation::Generation => CursorRecord {
                generation: Uuid::new_v4(),
                ..current
            },
            RetiredMutation::NewerRevision => CursorRecord {
                publication_revision: current.publication_revision + CURSOR_REVISION_STEP,
                ..current
            },
            RetiredMutation::SameRevisionConflict => CursorRecord {
                committed_bytes: current.committed_bytes - INITIAL_RETAINED_SEQUENCE,
                ..current
            },
            RetiredMutation::SameRecord => current,
        };
        fs::write(&isolated, encode_cursor(retired)).unwrap();

        let result = Journal::open(&fixture.moved_journal, fixture.journal.runtime_epoch());
        if matches!(mutation, RetiredMutation::SameRecord) {
            assert!(result.is_ok());
            assert!(!isolated.exists());
        } else {
            assert!(matches!(result, Err(JournalError::InvalidFormat)));
            assert!(isolated.exists());
        }
        fixture.assert_replacements_unchanged();
    }
}

#[test]
fn current_cursor_commit_must_stay_within_the_sealed_journal() {
    for mutation in [
        CurrentCommitMutation::BeforeSeal,
        CurrentCommitMutation::BeyondFile,
    ] {
        let root = private_root();
        let path = root.path().join(CURSOR_LOAD_FILE);
        let journal = Journal::open(&path, Uuid::new_v4()).unwrap();
        let generation = journal.header.generation;
        let sealed = journal.header.sealed_bytes;
        let file_len = journal.file.metadata().unwrap().len();
        drop(journal);
        let committed_bytes = match mutation {
            CurrentCommitMutation::BeforeSeal => sealed - INITIAL_RETAINED_SEQUENCE,
            CurrentCommitMutation::BeyondFile => file_len + INITIAL_RETAINED_SEQUENCE,
        };
        fs::write(
            cursor_path(&path),
            encode_cursor(CursorRecord {
                generation,
                committed_bytes,
                publication_revision: INITIAL_CURSOR_REVISION,
            }),
        )
        .unwrap();

        assert!(matches!(
            Journal::open(&path, Uuid::new_v4()),
            Err(JournalError::InvalidFormat)
        ));
    }
}

#[test]
fn cursor_publication_revision_exhaustion_is_reported_before_install() {
    let root = private_root();
    let mut journal = Journal::open(root.path().join(CURSOR_LOAD_FILE), Uuid::new_v4()).unwrap();
    journal.commit_cursor.record.publication_revision = u64::MAX;

    assert!(matches!(
        journal.commit_cursor.publish(journal.disk_bytes),
        Err(JournalError::CursorRevisionExhausted)
    ));
}

fn isolated_fixture() -> (RetainedFixture, PathBuf) {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    inject_parent_sync_failure();
    assert!(fixture.reconcile().is_err());
    let isolated =
        fixture.artifact(crate::paths::test_support::retained_isolation_artifact_prefix());
    (fixture, isolated)
}

fn active_cursor(path: &Path) -> CursorRecord {
    fs::read(path)
        .unwrap()
        .chunks_exact(COMMIT_CURSOR_BYTES)
        .filter_map(|bytes| decode_cursor(bytes).ok())
        .max_by_key(|record| record.publication_revision)
        .unwrap()
}

fn cursor_path(journal: &Path) -> PathBuf {
    let mut name = journal.file_name().map(OsString::from).unwrap();
    name.push(COMMIT_FILE_SUFFIX);
    journal.with_file_name(name)
}

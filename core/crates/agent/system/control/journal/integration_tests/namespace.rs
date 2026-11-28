use crate::{constants::*, fixture::*};
use uuid::Uuid;
use yolo_agent_control_journal::*;
use yolo_paths::directory::PrivateDirectory;

#[test]
fn singleton_and_explicit_creation_never_replace_an_initialized_journal() {
    let fixture = Fixture::new();
    let journal = fixture.create();
    let anchor = journal.anchor();
    assert!(matches!(fixture.reopen(anchor), Err(JournalError::Busy)));
    assert!(matches!(
        Journal::create_new(fixture.retained(), fixture.scope),
        Err(JournalError::Busy)
    ));
    drop(journal);
    assert!(matches!(
        Journal::create_new(fixture.retained(), fixture.scope),
        Err(JournalError::AlreadyExists)
    ));
    fixture.reopen(anchor).unwrap().verify().unwrap();
}

#[test]
fn missing_header_receipts_or_entire_history_never_reinitialize_on_reopen() {
    for removed in [
        vec![HEADER_FILE],
        vec![JOURNAL_FILE],
        vec![HEADER_FILE, JOURNAL_FILE],
    ] {
        let fixture = Fixture::new();
        let mut journal = fixture.create();
        let anchor = journal.anchor();
        drop(reserve(&mut journal, Uuid::new_v4()));
        drop(journal);
        for file in &removed {
            std::fs::remove_file(fixture.directory.join(file)).unwrap();
        }
        assert!(fixture.reopen(anchor).is_err());
        for file in &removed {
            assert!(!fixture.directory.join(file).exists());
        }
    }
}

#[test]
fn replaced_valid_header_is_rejected_by_persisted_external_anchor() {
    let fixture = Fixture::new();
    let mut journal = fixture.create();
    let anchor = journal.anchor();
    drop(reserve(&mut journal, Uuid::new_v4()));
    let path = fixture.directory.join(HEADER_FILE);
    let original = fixture.read(&path);
    fixture.write(&path, &original);
    assert_eq!(journal.verify(), Err(JournalError::Unavailable));
    assert!(matches!(
        journal.reserve(Uuid::new_v4(), digest()),
        Err(JournalError::Poisoned)
    ));
    drop(journal);
    assert!(matches!(
        fixture.reopen(anchor),
        Err(JournalError::ScopeMismatch)
    ));
}

#[test]
fn live_receipt_replacement_poisoning_cannot_be_reset_by_an_ordinary_rejection() {
    let fixture = Fixture::new();
    let mut journal = fixture.create();
    let request = Uuid::new_v4();
    drop(reserve(&mut journal, request));
    let path = fixture.directory.join(JOURNAL_FILE);
    fixture.write(&path, &fixture.read(&path));
    assert_eq!(
        journal.lookup(request, digest()),
        Err(JournalError::Unavailable)
    );
    assert_eq!(
        journal.lookup(Uuid::nil(), digest()),
        Err(JournalError::InvalidRequest)
    );
    assert!(matches!(
        journal.reserve(Uuid::new_v4(), digest()),
        Err(JournalError::Poisoned)
    ));
}

#[test]
fn directory_replacement_and_fresh_same_scope_journal_cannot_adopt_old_anchor() {
    let fixture = Fixture::new();
    let mut original = fixture.create();
    let anchor = original.anchor();
    let request = Uuid::new_v4();
    drop(reserve(&mut original, request));
    let retired = fixture.root.path().canonicalize().unwrap().join(RETIRED);
    std::fs::rename(&fixture.directory, &retired).unwrap();
    PrivateDirectory::ensure(&fixture.directory).unwrap();
    let marker = fixture.directory.join(MARKER);
    std::fs::write(&marker, MARKER_BYTES).unwrap();
    assert!(matches!(
        original.reserve(Uuid::new_v4(), digest()),
        Err(JournalError::Unavailable)
    ));
    assert!(!fixture.directory.join(JOURNAL_FILE).exists());
    drop(original);
    let replacement = fixture.create();
    assert_ne!(replacement.anchor(), anchor);
    drop(replacement);
    assert!(matches!(
        fixture.reopen(anchor),
        Err(JournalError::ScopeMismatch)
    ));
    assert_eq!(std::fs::read(marker).unwrap(), MARKER_BYTES);
}

#[test]
fn total_erasure_followed_by_explicit_new_initialization_still_cannot_match_retained_anchor() {
    let fixture = Fixture::new();
    let mut first = fixture.create();
    let expected = first.anchor();
    drop(reserve(&mut first, Uuid::new_v4()));
    drop(first);
    std::fs::remove_file(fixture.directory.join(HEADER_FILE)).unwrap();
    std::fs::remove_file(fixture.directory.join(JOURNAL_FILE)).unwrap();
    // Even an incorrect caller that initializes again in the same directory cannot
    // reopen that new empty journal using the trusted runtime's original anchor.
    let fresh = fixture.create();
    let replacement = fresh.anchor();
    assert_ne!(replacement.journal_id(), expected.journal_id());
    drop(fresh);
    assert!(matches!(
        fixture.reopen(expected),
        Err(JournalError::ScopeMismatch)
    ));
    fixture.reopen(replacement).unwrap().verify().unwrap();
}

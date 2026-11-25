use super::{constants::*, fixture::*};
use crate::{HEADER_FILE, JOURNAL_FILE, Journal, JournalError, record::Header};
use std::{
    cell::RefCell,
    ffi::{CString, OsStr},
    os::unix::ffi::OsStrExt,
    rc::Rc,
};
use uuid::Uuid;

#[test]
fn replacing_header_between_initialization_commits_preserves_replacement_and_grants_no_journal() {
    for same_contents in [false, true] {
        let fixture = Fixture::new();
        let namespace = fixture.namespace();
        let record_path = fixture.directory.join(JOURNAL_FILE);
        let record_entry = namespace.file(&record_path).unwrap();
        let retired_name = yolo_paths::test_support::retired_storage_artifact_name(
            &CString::new(JOURNAL_FILE).unwrap(),
        )
        .unwrap();
        let staging_path = fixture
            .directory
            .join(OsStr::from_bytes(retired_name.as_bytes()));
        let staging = namespace.file(&staging_path).unwrap();
        yolo_paths::test_support::write_private_staging(
            &staging,
            STAGING_CONTENT,
            yolo_paths::constants::MAX_DURABLE_JSON_BYTES,
        )
        .unwrap();
        let replacement = Rc::new(RefCell::new(None));
        let witness = replacement.clone();
        let header_path = fixture.directory.join(HEADER_FILE);
        let before_commit_path = header_path.clone();
        let before_commit_record = record_path.clone();
        // Header creation has no retired slot. Receipt staging does, so this hook runs
        // only after the header was durably published and before its receipt fence.
        let _hook =
            yolo_paths::test_support::install_after_move_sync_hook(&record_entry, move || {
                assert!(!before_commit_record.exists());
                let (header, original): (Header, _) =
                    namespace.read_json_with_identity(&before_commit_path)?;
                let expected = header.anchor(original);
                let mut replacement_header = header;
                if !same_contents {
                    replacement_header.journal_id = Uuid::new_v4();
                }
                namespace.write_json(&before_commit_path, &replacement_header)?;
                let (saved, current): (Header, _) =
                    namespace.read_json_with_identity(&before_commit_path)?;
                assert_eq!(saved, replacement_header);
                assert_ne!(current, original);
                *witness.borrow_mut() = Some((expected, saved, current));
                Ok(())
            });
        let failure = Journal::create_new(fixture.retained(), fixture.scope)
            .err()
            .expect(EXPECTED_REJECTION);
        assert_eq!(failure, JournalError::OutcomeUnknown);
        let (expected, header, replacement_identity) = replacement.borrow_mut().take().unwrap();
        let (preserved, identity): (Header, _) = fixture
            .namespace()
            .read_json_with_identity(&header_path)
            .unwrap();
        assert_eq!((preserved, identity), (header, replacement_identity));
        assert!(!record_path.exists());
        assert!(matches!(
            fixture.reopen(expected),
            Err(JournalError::ScopeMismatch)
        ));
        assert!(matches!(
            Journal::create_new(fixture.retained(), fixture.scope),
            Err(JournalError::AlreadyExists)
        ));
    }
}

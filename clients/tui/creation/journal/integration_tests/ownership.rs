use super::{filesystem::*, model::*};
use crate::*;
use std::{
    ffi::CString,
    os::unix::{ffi::OsStrExt, fs::PermissionsExt},
};
use yolo_hub_identity::HubId;
use yolo_paths::directory::PrivateDirectory;

#[test]
fn second_owner_wrong_entry_and_copied_namespace_are_rejected() {
    let fixture = Fixture::new();
    let mut journal = fixture.open();
    journal.retain_confirmed(pending(HubId::new())).unwrap();
    assert!(CreationJournal::open(fixture.parent(), fixture.entry).is_err());
    drop(journal);
    assert!(CreationJournal::open(fixture.parent(), HubId::new()).is_err());
    let other = fixture
        .root
        .path()
        .canonicalize()
        .unwrap()
        .join(OTHER_DIRECTORY);
    let parent = PrivateDirectory::ensure(&other).unwrap();
    yolo_paths::atomic_write_json(&other.join(JOURNAL_FILE), &fixture.stored()).unwrap();
    assert!(CreationJournal::open(parent, fixture.entry).is_err());
    assert_eq!(
        fixture.open().pending().unwrap().len(),
        CAPACITY_SENTINEL as usize
    );
}

#[test]
fn malformed_schema_unknown_fields_and_duplicate_receipts_are_never_reset() {
    let fixture = Fixture::new();
    let mut journal = fixture.open();
    journal.retain_confirmed(pending(HubId::new())).unwrap();
    drop(journal);
    let original = fixture.stored();
    let variants = [
        {
            let mut v = original.clone();
            v[SCHEMA_FIELD] = (JOURNAL_SCHEMA_VERSION + SCHEMA_STEP).into();
            v
        },
        {
            let mut v = original.clone();
            v[UNKNOWN_FIELD] = true.into();
            v
        },
        {
            let mut v = original.clone();
            v[REVISION_FIELD] = u64::default().into();
            v
        },
        {
            let mut v = original;
            let first = v[PENDING_FIELD][FIRST_INDEX].clone();
            v[PENDING_FIELD].as_array_mut().unwrap().push(first);
            v
        },
    ];
    for value in variants {
        fixture.replace(&value);
        let before = std::fs::read(fixture.file()).unwrap();
        assert!(CreationJournal::open(fixture.parent(), fixture.entry).is_err());
        assert_eq!(std::fs::read(fixture.file()).unwrap(), before);
    }
    std::fs::write(fixture.file(), INVALID_JSON).unwrap();
    assert!(CreationJournal::open(fixture.parent(), fixture.entry).is_err());
    assert_eq!(std::fs::read(fixture.file()).unwrap(), INVALID_JSON);
}

#[test]
fn record_byte_budget_is_checked_before_json_decoding() {
    let fixture = Fixture::new();
    drop(fixture.open());
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(fixture.file())
        .unwrap();
    let oversized = yolo_paths::constants::MAX_DURABLE_JSON_BYTES + CAPACITY_SENTINEL;
    file.set_len(oversized).unwrap();
    assert!(CreationJournal::open(fixture.parent(), fixture.entry).is_err());
    assert_eq!(file.metadata().unwrap().len(), oversized);
}

#[test]
fn unsafe_permissions_hardlinks_and_fifo_cannot_be_adopted() {
    let fixture = Fixture::new();
    drop(fixture.open());
    std::fs::set_permissions(fixture.file(), std::fs::Permissions::from_mode(PUBLIC_MODE)).unwrap();
    assert!(CreationJournal::open(fixture.parent(), fixture.entry).is_err());
    std::fs::set_permissions(
        fixture.file(),
        std::fs::Permissions::from_mode(PRIVATE_MODE),
    )
    .unwrap();
    let other = fixture.root.path().join(OTHER_DIRECTORY);
    std::fs::hard_link(fixture.file(), &other).unwrap();
    assert!(CreationJournal::open(fixture.parent(), fixture.entry).is_err());
    std::fs::remove_file(other).unwrap();
    assert!(fixture.open().pending().unwrap().is_empty());
    std::fs::remove_file(fixture.file()).unwrap();
    let path = CString::new(fixture.file().as_os_str().as_bytes()).unwrap();
    assert_eq!(
        unsafe { libc::mkfifo(path.as_ptr(), PRIVATE_MODE as libc::mode_t) },
        libc::c_int::default()
    );
    assert!(CreationJournal::open(fixture.parent(), fixture.entry).is_err());
}

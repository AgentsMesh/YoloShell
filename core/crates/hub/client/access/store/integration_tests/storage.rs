use crate::{constants::*, fixture::*, model::*};
use std::{fs, os::unix::fs::PermissionsExt};

#[test]
fn missing_or_symlinked_record_cannot_be_recreated_and_the_link_target_is_untouched() {
    for symlink in [false, true] {
        let fixture = Fixture::new();
        let mut store = fixture.open(Some(configuration())).unwrap();
        let original = store.current().unwrap();
        fs::rename(fixture.record(), fixture.sibling(RETIRED_RECORD)).unwrap();
        let target = fixture.sibling(SYMLINK_TARGET);
        if symlink {
            write_new(&target, INVALID_BYTES);
            std::os::unix::fs::symlink(&target, fixture.record()).unwrap();
        }
        assert_poisoned(&mut store, &original);
        if symlink {
            assert_eq!(fs::read(target).unwrap(), INVALID_BYTES);
            drop(store);
            assert!(fixture.open(None).is_err());
        } else {
            assert!(!fixture.record().exists());
        }
    }
}

#[test]
fn nonprivate_file_or_directory_permissions_revoke_access_even_after_permissions_are_restored() {
    for directory in [false, true] {
        let fixture = Fixture::new();
        let mut store = fixture.open(Some(configuration())).unwrap();
        let original = store.current().unwrap();
        let bytes = fixture.bytes();
        let (path, unsafe_mode, private_mode) = if directory {
            (
                fixture.path.clone(),
                PUBLIC_DIRECTORY_MODE,
                PRIVATE_DIRECTORY_MODE,
            )
        } else {
            (fixture.record(), PUBLIC_FILE_MODE, PRIVATE_FILE_MODE)
        };
        fs::set_permissions(&path, fs::Permissions::from_mode(unsafe_mode)).unwrap();
        assert_poisoned(&mut store, &original);
        fs::set_permissions(path, fs::Permissions::from_mode(private_mode)).unwrap();
        assert_poisoned(&mut store, &original);
        assert_eq!(fixture.bytes(), bytes);
        drop(store);
        assert_eq!(fixture.open(None).unwrap().current().unwrap(), original);
    }
}

#[test]
fn a_second_record_hardlink_cannot_be_used_as_live_authority() {
    let fixture = Fixture::new();
    let mut store = fixture.open(Some(configuration())).unwrap();
    let original = store.current().unwrap();
    let bytes = fixture.bytes();
    let link = fixture.sibling(HARDLINK_TARGET);
    fs::hard_link(fixture.record(), &link).unwrap();
    assert_poisoned(&mut store, &original);
    fs::remove_file(link).unwrap();
    assert_poisoned(&mut store, &original);
    assert_eq!(fixture.bytes(), bytes);
    drop(store);
    assert_eq!(fixture.open(None).unwrap().current().unwrap(), original);
}

#[test]
fn corrupt_private_records_are_rejected_on_restart_without_reset_or_rewrite() {
    let mutations: &[fn(&mut serde_json::Value)] = &[
        |record| record[SCHEMA_FIELD] = FUTURE_SCHEMA.into(),
        |record| record[CONFIGURATION_FIELD][SCHEMA_FIELD] = INVALID_SCHEMA.into(),
        |record| record[CONFIGURATION_FIELD][REVISION_FIELD] = u64::MIN.into(),
        |record| record[CONFIGURATION_FIELD][ENTRY_HUB_FIELD] = other_hub().to_string().into(),
        |record| record[UNKNOWN_FIELD] = true.into(),
        |record| record[CONFIGURATION_FIELD][UNKNOWN_FIELD] = true.into(),
        |record| {
            record.as_object_mut().unwrap().remove(DIRECTORY_FIELD);
        },
    ];
    for change in mutations {
        let fixture = Fixture::new();
        drop(fixture.open(Some(configuration())).unwrap());
        let mut record: serde_json::Value = serde_json::from_slice(&fixture.bytes()).unwrap();
        change(&mut record);
        let bytes = serde_json::to_vec(&record).unwrap();
        fs::write(fixture.record(), &bytes).unwrap();
        assert!(fixture.open(None).is_err());
        assert_eq!(fixture.bytes(), bytes);
    }
    let fixture = Fixture::new();
    drop(fixture.open(Some(configuration())).unwrap());
    fs::write(fixture.record(), INVALID_BYTES).unwrap();
    assert!(fixture.open(None).is_err());
    assert_eq!(fixture.bytes(), INVALID_BYTES);
}

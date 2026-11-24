use super::developer_release_fixture_archive::{
    DistributedArchiveKind, candidate_archive, distributed_archive,
};
use crate::developer_release_constants::{
    DEVELOPER_RELEASE_INVENTORY_ERROR, MAX_NOTARY_TICKET_BYTES, NOTARY_TICKET_ARCHIVE_PATH,
    SIGNED_ARCHIVE_PATHS,
};
use crate::developer_release_inventory::{validate_signed_transition, validate_transition};
use yolo_release_archive_audit::audit_bytes;
use yolo_release_archive_audit::constants::ROOT_DIRECTORY_ENTRY;
use yolo_release_archive_audit_test_support::{FixtureEntry, build};
use yolo_release_foundation::digest::bytes_sha256;

const CONTENTS_ENTRY: &[u8] = b"Yolo.app/Contents/";
const PAYLOAD_ENTRY: &[u8] = b"Yolo.app/Contents/payload";
const UNEXPECTED_EXECUTABLES: &[&str] = &[
    "Yolo.app/Contents/MacOS/yolo",
    "Yolo.app/Contents/Helpers/yolo-host",
    "Yolo.app/Contents/Helpers/nested/extra-runtime",
];

#[test]
fn transition_allows_only_the_new_notarization_ticket() {
    validate_transition(&candidate(), &distributed()).unwrap();
}

#[test]
fn signed_transition_allows_signatures_but_not_a_premature_ticket() {
    validate_signed_transition(&candidate(), &candidate()).unwrap();
    assert!(validate_signed_transition(&candidate(), &distributed()).is_err());
}

#[test]
fn immutable_change_with_the_same_size_and_crc_is_rejected_by_sha256() {
    let before = candidate();
    let mut changed = distributed();
    changed
        .entries
        .iter_mut()
        .find(|entry| entry.path.as_bytes() == PAYLOAD_ENTRY)
        .unwrap()
        .sha256 = bytes_sha256(b"same-size CRC collision");
    let error = super::support::error_text(validate_transition(&before, &changed));
    assert!(error.contains(DEVELOPER_RELEASE_INVENTORY_ERROR), "{error}");
}

#[test]
fn missing_empty_or_extra_ticket_state_is_rejected() {
    assert!(validate_transition(&candidate(), &candidate()).is_err());
    let empty_ticket = build(&[
        FixtureEntry::directory(ROOT_DIRECTORY_ENTRY.as_bytes()),
        FixtureEntry::directory(CONTENTS_ENTRY),
        FixtureEntry::regular(PAYLOAD_ENTRY),
        FixtureEntry::regular(NOTARY_TICKET_ARCHIVE_PATH.as_bytes()),
    ]);
    assert!(validate_transition(&candidate(), &audit_bytes(&empty_ticket.bytes).unwrap()).is_err());
}

#[test]
fn duplicate_inventory_entries_and_already_stapled_candidates_are_rejected() {
    let mut duplicated = candidate();
    duplicated
        .entries
        .push(duplicated.entries.first().unwrap().clone());
    assert!(validate_signed_transition(&duplicated, &candidate()).is_err());
    assert!(validate_signed_transition(&candidate(), &duplicated).is_err());
    assert!(validate_transition(&distributed(), &distributed()).is_err());
}

#[test]
fn neither_executable_directory_may_hide_an_extra_or_retired_binary() {
    for path in UNEXPECTED_EXECUTABLES {
        let mut inventory = candidate();
        let mut unexpected = inventory
            .entries
            .iter()
            .find(|entry| SIGNED_ARCHIVE_PATHS.contains(&entry.path.as_str()))
            .unwrap()
            .clone();
        unexpected.path = (*path).into();
        inventory.entries.push(unexpected);
        assert_eq!(
            validate_signed_transition(&inventory, &inventory)
                .unwrap_err()
                .to_string(),
            DEVELOPER_RELEASE_INVENTORY_ERROR
        );
    }
}

#[test]
fn signing_may_change_executable_bytes_but_never_type_or_permissions() {
    let before = candidate();
    for mutate in [
        |entry: &mut yolo_release_archive_audit::model::ArchiveEntry| {
            entry.mode = yolo_release_archive_audit::constants::REGULAR_RESOURCE_MODE;
        },
        |entry: &mut yolo_release_archive_audit::model::ArchiveEntry| {
            entry.is_directory = true;
        },
    ] {
        let mut changed = candidate();
        let executable = changed
            .entries
            .iter_mut()
            .find(|entry| SIGNED_ARCHIVE_PATHS.contains(&entry.path.as_str()))
            .unwrap();
        mutate(executable);
        assert_eq!(
            validate_signed_transition(&before, &changed)
                .unwrap_err()
                .to_string(),
            DEVELOPER_RELEASE_INVENTORY_ERROR
        );
    }
}

#[test]
fn stapled_ticket_is_a_bounded_regular_resource() {
    let mutations: &[fn(&mut yolo_release_archive_audit::model::ArchiveEntry)] = &[
        |entry| entry.is_directory = true,
        |entry| entry.mode = yolo_release_archive_audit::constants::EXECUTABLE_MODE,
        |entry| entry.uncompressed_size = MAX_NOTARY_TICKET_BYTES + 1,
    ];
    for mutate in mutations {
        let mut changed = distributed();
        let ticket = changed
            .entries
            .iter_mut()
            .find(|entry| entry.path == NOTARY_TICKET_ARCHIVE_PATH)
            .unwrap();
        mutate(ticket);
        assert_eq!(
            validate_transition(&candidate(), &changed)
                .unwrap_err()
                .to_string(),
            DEVELOPER_RELEASE_INVENTORY_ERROR
        );
    }
}

fn candidate() -> yolo_release_archive_audit::model::ZipInventory {
    let bytes = candidate_archive();
    audit_bytes(&bytes).unwrap()
}

fn distributed() -> yolo_release_archive_audit::model::ZipInventory {
    let bytes = distributed_archive(DistributedArchiveKind::Exact);
    audit_bytes(&bytes).unwrap()
}

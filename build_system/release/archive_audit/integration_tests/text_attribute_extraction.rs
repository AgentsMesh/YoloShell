#[path = "../test_support/text_attribute_constants.rs"]
mod constants;
#[path = "../test_support/ditto.rs"]
mod ditto;
use constants::*;
use std::{fs, os::unix::fs::PermissionsExt};
use yolo_release_archive_audit::{audit_archive_path, constants::*};
use yolo_release_archive_audit_test_support::{
    FixtureEntry, PAYLOAD_INDEX, PAYLOAD_PATH, build, set_u16,
};
use yolo_release_foundation::digest::bytes_sha256;

#[test]
fn ditto_preserves_raw_mixed_newlines_and_nul_with_stored_or_deflated_text_attributes() {
    let mut baseline = None;
    for method in [STORED_METHOD, DEFLATE_METHOD] {
        for attribute in [EMPTY_FIELD_LENGTH, TEXT_INTERNAL_ATTRIBUTE] {
            let root = tempfile::tempdir().unwrap();
            let archive = root.path().join(ARCHIVE_FILE);
            let destination = root.path().join(EXTRACTION_DIRECTORY);
            let mut fixture = build(&[
                FixtureEntry::directory(ROOT_DIRECTORY_ENTRY.as_bytes()),
                entry(method),
            ]);
            let central = fixture.entries[PAYLOAD_INDEX].central;
            set_u16(
                &mut fixture.bytes,
                central + CENTRAL_INTERNAL_OFFSET,
                attribute,
            );
            fs::write(&archive, &fixture.bytes).unwrap();
            let inventory = audit_archive_path(&archive).unwrap();
            let expected = &inventory.entries[PAYLOAD_INDEX];
            assert_eq!(expected.sha256, bytes_sha256(MIXED_PAYLOAD));
            assert_eq!(expected.uncompressed_size, MIXED_PAYLOAD.len() as u64);
            ditto::extract(&archive, &destination);
            let extracted = destination.join(std::str::from_utf8(PAYLOAD_PATH).unwrap());
            let bytes = fs::read(&extracted).unwrap();
            assert_eq!(bytes, MIXED_PAYLOAD);
            assert_eq!(bytes_sha256(&bytes), expected.sha256);
            assert_eq!(fs::read(&archive).unwrap(), fixture.bytes);
            let metadata = fs::symlink_metadata(&extracted).unwrap();
            assert!(metadata.is_file());
            assert_eq!(
                metadata.permissions().mode() as u16 & UNIX_PERMISSION_MASK,
                REGULAR_RESOURCE_MODE
            );
            let observed = (bytes, expected.sha256.clone());
            if let Some(baseline) = &baseline {
                assert_eq!(baseline, &observed);
            } else {
                baseline = Some(observed);
            }
        }
    }
}

fn entry(method: u16) -> FixtureEntry {
    let mut entry = FixtureEntry::regular(PAYLOAD_PATH);
    entry.method = method;
    entry.crc32 = crc32fast::hash(MIXED_PAYLOAD);
    entry.uncompressed_size = MIXED_PAYLOAD.len().try_into().unwrap();
    entry.payload = if method == DEFLATE_METHOD {
        let length = u16::try_from(MIXED_PAYLOAD.len()).unwrap();
        let mut block = vec![RAW_DEFLATE_FINAL_STORED_BLOCK];
        block.extend_from_slice(&length.to_le_bytes());
        block.extend_from_slice(&(!length).to_le_bytes());
        block.extend_from_slice(MIXED_PAYLOAD);
        block
    } else {
        MIXED_PAYLOAD.to_vec()
    };
    entry
}

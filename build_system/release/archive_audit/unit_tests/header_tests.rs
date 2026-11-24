use yolo_release_archive_audit::constants::*;
use yolo_release_archive_audit_test_support::*;

const ENCRYPTED_FLAG: u16 = 0x0001;
const DATA_DESCRIPTOR_FLAG: u16 = 0x0008;
const UTF8_NAME_FLAG: u16 = 0x0800;
const UNSUPPORTED_METHOD: u16 = 99;
const NONEMPTY_UNSUPPORTED_FIELD: u16 = 1;
const MISMATCHED_FIELD: u16 = 1;
const MISMATCHED_CRC32: u32 = 1;
const UNSUPPORTED_INTERNAL_ATTRIBUTE: u16 = 0x0002;

#[test]
fn valid_archive_produces_bounded_inventory() {
    let fixture = valid_archive();
    let inventory = yolo_release_archive_audit::audit_bytes(&fixture.bytes).unwrap();
    assert_eq!(inventory.entries.len(), 2);
    assert_eq!(inventory.entries[ROOT_INDEX].path, ROOT_DIRECTORY_ENTRY);
    assert!(inventory.entries[ROOT_INDEX].is_directory);
    assert_eq!(
        inventory.entries[PAYLOAD_INDEX].path.as_bytes(),
        PAYLOAD_PATH
    );
}

#[test]
fn central_safe_local_evil_name_is_rejected() {
    let mut entry = FixtureEntry::regular(PAYLOAD_PATH);
    entry.local_name = b"Yolo.app/../evil".to_vec();
    assert_rejected(
        &build(&[
            FixtureEntry::directory(ROOT_DIRECTORY_ENTRY.as_bytes()),
            entry,
        ])
        .bytes,
    );
}

#[test]
fn unsupported_flags_methods_extra_fields_and_comments_are_rejected() {
    for unsupported in [ENCRYPTED_FLAG, DATA_DESCRIPTOR_FLAG, UTF8_NAME_FLAG] {
        let mut flags = valid_archive();
        let central = flags.entries[PAYLOAD_INDEX].central;
        set_u16(
            &mut flags.bytes,
            central + CENTRAL_FLAGS_OFFSET,
            unsupported,
        );
        assert_rejected(&flags.bytes);
    }

    let mut method = valid_archive();
    let central = method.entries[PAYLOAD_INDEX].central;
    set_u16(
        &mut method.bytes,
        central + CENTRAL_METHOD_OFFSET,
        UNSUPPORTED_METHOD,
    );
    assert_rejected(&method.bytes);

    let mut extra = valid_archive();
    let central = extra.entries[PAYLOAD_INDEX].central;
    set_u16(
        &mut extra.bytes,
        central + CENTRAL_EXTRA_LENGTH_OFFSET,
        NONEMPTY_UNSUPPORTED_FIELD,
    );
    assert_rejected(&extra.bytes);

    let mut comment = valid_archive();
    let central = comment.entries[PAYLOAD_INDEX].central;
    set_u16(
        &mut comment.bytes,
        central + CENTRAL_COMMENT_LENGTH_OFFSET,
        NONEMPTY_UNSUPPORTED_FIELD,
    );
    assert_rejected(&comment.bytes);

    let mut internal = valid_archive();
    let central = internal.entries[PAYLOAD_INDEX].central;
    set_u16(
        &mut internal.bytes,
        central + CENTRAL_INTERNAL_OFFSET,
        UNSUPPORTED_INTERNAL_ATTRIBUTE,
    );
    assert_rejected(&internal.bytes);
}

#[test]
fn local_and_central_security_fields_are_bound() {
    for offset in [
        LOCAL_NEEDED_OFFSET,
        LOCAL_FLAGS_OFFSET,
        LOCAL_METHOD_OFFSET,
        LOCAL_TIME_OFFSET,
        LOCAL_DATE_OFFSET,
    ] {
        let mut fixture = valid_archive();
        let local = fixture.entries[PAYLOAD_INDEX].local;
        set_u16(&mut fixture.bytes, local + offset, MISMATCHED_FIELD);
        assert_rejected(&fixture.bytes);
    }
    let mut crc = valid_archive();
    let local = crc.entries[PAYLOAD_INDEX].local;
    set_u32(&mut crc.bytes, local + LOCAL_CRC_OFFSET, MISMATCHED_CRC32);
    assert_rejected(&crc.bytes);
}

#[test]
fn local_extra_field_and_sizes_are_bound() {
    let mut extra = valid_archive();
    let local = extra.entries[PAYLOAD_INDEX].local;
    set_u16(
        &mut extra.bytes,
        local + LOCAL_EXTRA_LENGTH_OFFSET,
        NONEMPTY_UNSUPPORTED_FIELD,
    );
    assert_rejected(&extra.bytes);
    let mut size = valid_archive();
    let local = size.entries[PAYLOAD_INDEX].local;
    set_u32(
        &mut size.bytes,
        local + LOCAL_UNCOMPRESSED_OFFSET,
        u32::from(MISMATCHED_FIELD),
    );
    assert_rejected(&size.bytes);
}

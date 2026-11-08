use yolo_release_archive_audit::constants::*;
use yolo_release_archive_audit_test_support::{
    FixtureEntry, PAYLOAD_INDEX, assert_accepted, assert_rejected, build, set_u32,
};

fn archive_with(entry: FixtureEntry) -> Vec<u8> {
    build(&[
        FixtureEntry::directory(ROOT_DIRECTORY_ENTRY.as_bytes()),
        entry,
    ])
    .bytes
}

#[test]
fn supported_regular_and_directory_modes_are_accepted() {
    let mut executable = FixtureEntry::regular(b"Yolo.app/Contents/MacOS/Yolo");
    executable.unix_mode = UNIX_REGULAR_TYPE | EXECUTABLE_MODE;
    assert_accepted(&archive_with(executable));
}

#[test]
fn symbolic_fifo_socket_device_and_unknown_types_are_rejected() {
    let types = [0o120000, 0o010000, 0o140000, 0o060000, 0_u16];
    for file_type in types {
        let mut entry = FixtureEntry::regular(b"Yolo.app/Contents/unsafe");
        entry.unix_mode = file_type | REGULAR_RESOURCE_MODE;
        assert_rejected(&archive_with(entry));
    }
}

#[test]
fn directory_suffix_and_type_must_agree() {
    assert_rejected(&archive_with(FixtureEntry::regular(
        b"Yolo.app/Contents/regular/",
    )));
    assert_rejected(&archive_with(FixtureEntry::directory(
        b"Yolo.app/Contents/directory",
    )));
}

#[test]
fn directory_payload_special_bits_and_unknown_modes_are_rejected() {
    let mut payload_directory = FixtureEntry::directory(b"Yolo.app/Contents/data/");
    payload_directory.payload.push(0);
    assert_rejected(&archive_with(payload_directory));
    for mode in [0o600, 0o4755, 0o2755, 0o1755] {
        let mut entry = FixtureEntry::regular(b"Yolo.app/Contents/unsafe");
        entry.unix_mode = UNIX_REGULAR_TYPE | mode;
        assert_rejected(&archive_with(entry));
    }
}

#[test]
fn directory_payload_contract_rejects_compression_crc_and_nonempty_stored_data() {
    const DIRECTORY_PATH: &[u8] = b"Yolo.app/Contents/data/";
    const NONEMPTY_BYTES: u32 = 1;
    for mutate in [
        |entry: &mut FixtureEntry| entry.method = DEFLATE_METHOD,
        |entry: &mut FixtureEntry| entry.crc32 = NONEMPTY_BYTES,
        |entry: &mut FixtureEntry| {
            entry.payload.push(0);
            entry.uncompressed_size = NONEMPTY_BYTES;
        },
    ] {
        let mut directory = FixtureEntry::directory(DIRECTORY_PATH);
        mutate(&mut directory);
        assert_eq!(
            yolo_release_archive_audit::audit_bytes(&archive_with(directory))
                .unwrap_err()
                .to_string(),
            "ZIP directory payload must be empty and stored"
        );
    }
}

#[test]
fn non_unix_origin_is_rejected() {
    let mut entry = FixtureEntry::regular(b"Yolo.app/Contents/dos");
    entry.origin = 0;
    assert_rejected(&archive_with(entry));
}

#[test]
fn unix_type_and_dos_attributes_must_agree() {
    let mut regular = build(&[
        FixtureEntry::directory(ROOT_DIRECTORY_ENTRY.as_bytes()),
        FixtureEntry::regular(b"Yolo.app/Contents/regular"),
    ]);
    let central = regular.entries[PAYLOAD_INDEX].central;
    let attributes = u32::from(UNIX_REGULAR_TYPE | REGULAR_RESOURCE_MODE) << u16::BITS
        | u32::from(DOS_DIRECTORY_ATTRIBUTE);
    set_u32(
        &mut regular.bytes,
        central + CENTRAL_EXTERNAL_OFFSET,
        attributes,
    );
    assert_rejected(&regular.bytes);

    let mut directory = build(&[
        FixtureEntry::directory(ROOT_DIRECTORY_ENTRY.as_bytes()),
        FixtureEntry::directory(b"Yolo.app/Contents/directory/"),
    ]);
    let central = directory.entries[PAYLOAD_INDEX].central;
    let attributes = u32::from(UNIX_DIRECTORY_TYPE | DIRECTORY_MODE) << u16::BITS;
    set_u32(
        &mut directory.bytes,
        central + CENTRAL_EXTERNAL_OFFSET,
        attributes,
    );
    assert_rejected(&directory.bytes);
}

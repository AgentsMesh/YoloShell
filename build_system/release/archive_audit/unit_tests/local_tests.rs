use yolo_release_archive_audit::constants::*;
use yolo_release_archive_audit_test_support::*;

const FIRST_PATH: &[u8] = b"Yolo.app/Contents/first";
const SECOND_PATH: &[u8] = b"Yolo.app/Contents/second";

#[test]
fn overlapping_local_spans_are_rejected() {
    let mut fixture = build(&[
        FixtureEntry::directory(ROOT_DIRECTORY_ENTRY.as_bytes()),
        FixtureEntry::regular(FIRST_PATH),
        FixtureEntry::regular(SECOND_PATH),
    ]);
    let local = fixture.entries[PAYLOAD_INDEX].local;
    let central = fixture.entries[PAYLOAD_INDEX].central;
    set_u32(&mut fixture.bytes, local + LOCAL_COMPRESSED_OFFSET, 1);
    set_u32(&mut fixture.bytes, local + LOCAL_UNCOMPRESSED_OFFSET, 1);
    set_u32(&mut fixture.bytes, central + CENTRAL_COMPRESSED_OFFSET, 1);
    set_u32(&mut fixture.bytes, central + CENTRAL_UNCOMPRESSED_OFFSET, 1);
    let overlapped_byte = LOCAL_SIGNATURE.to_le_bytes()[SIGNATURE_OFFSET];
    let overlapped_crc32 = crc32fast::hash(&[overlapped_byte]);
    set_u32(
        &mut fixture.bytes,
        local + LOCAL_CRC_OFFSET,
        overlapped_crc32,
    );
    set_u32(
        &mut fixture.bytes,
        central + CENTRAL_CRC_OFFSET,
        overlapped_crc32,
    );
    assert_rejected(&fixture.bytes);
}

#[test]
fn unknown_gap_between_local_entries_is_rejected() {
    let mut first = FixtureEntry::regular(FIRST_PATH);
    first.payload.push(0);
    let mut fixture = build(&[
        FixtureEntry::directory(ROOT_DIRECTORY_ENTRY.as_bytes()),
        first,
        FixtureEntry::regular(SECOND_PATH),
    ]);
    let local = fixture.entries[PAYLOAD_INDEX].local;
    let central = fixture.entries[PAYLOAD_INDEX].central;
    set_u32(&mut fixture.bytes, local + LOCAL_COMPRESSED_OFFSET, 0);
    set_u32(&mut fixture.bytes, local + LOCAL_UNCOMPRESSED_OFFSET, 0);
    set_u32(&mut fixture.bytes, central + CENTRAL_COMPRESSED_OFFSET, 0);
    set_u32(&mut fixture.bytes, central + CENTRAL_UNCOMPRESSED_OFFSET, 0);
    assert_rejected(&fixture.bytes);
}

#[test]
fn archive_preamble_is_rejected_even_with_adjusted_offsets() {
    let mut fixture = valid_archive();
    fixture.bytes.insert(0, 0);
    for entry in &fixture.entries {
        let central = entry.central + 1;
        let shifted_local = (entry.local + 1) as u32;
        set_u32(
            &mut fixture.bytes,
            central + CENTRAL_LOCAL_OFFSET,
            shifted_local,
        );
    }
    let shifted_eocd = fixture.eocd_offset + 1;
    set_u32(
        &mut fixture.bytes,
        shifted_eocd + EOCD_CENTRAL_OFFSET_OFFSET,
        (fixture.central_offset + 1) as u32,
    );
    assert_rejected(&fixture.bytes);
}

#[test]
fn central_local_offset_into_data_is_rejected() {
    let mut fixture = valid_archive();
    let central = fixture.entries[PAYLOAD_INDEX].central;
    set_u32(&mut fixture.bytes, central + CENTRAL_LOCAL_OFFSET, 1);
    assert_rejected(&fixture.bytes);
}

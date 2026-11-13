use yolo_release_archive_audit::constants::{EMPTY_CRC32, ROOT_DIRECTORY_ENTRY};
use yolo_release_archive_audit_test_support::{
    FixtureEntry, assert_accepted, assert_rejected, build,
};

const PAYLOAD_PATH: &[u8] = b"Yolo.app/Contents/deflated";
const HELLO_RAW_DEFLATE: &[u8] = b"\xcbH\xcd\xc9\xc9\x07\x00";
const HELLO_SIZE: u32 = 5;
const HELLO_CRC32: u32 = 0x3610_a686;
const WRONG_SIZE: u32 = HELLO_SIZE - 1;
const WRONG_CRC32: u32 = HELLO_CRC32 ^ 1;
const TRAILING_DEFLATE_BYTE: u8 = 0;

fn archive_with(entry: FixtureEntry) -> Vec<u8> {
    build(&[
        FixtureEntry::directory(ROOT_DIRECTORY_ENTRY.as_bytes()),
        entry,
    ])
    .bytes
}

#[test]
fn actual_raw_deflate_stream_size_and_crc_are_verified() {
    let valid = FixtureEntry::deflated(PAYLOAD_PATH, HELLO_RAW_DEFLATE, HELLO_SIZE, HELLO_CRC32);
    assert_accepted(&archive_with(valid));

    let wrong_size =
        FixtureEntry::deflated(PAYLOAD_PATH, HELLO_RAW_DEFLATE, WRONG_SIZE, HELLO_CRC32);
    assert_rejected(&archive_with(wrong_size));

    let wrong_crc =
        FixtureEntry::deflated(PAYLOAD_PATH, HELLO_RAW_DEFLATE, HELLO_SIZE, WRONG_CRC32);
    assert_rejected(&archive_with(wrong_crc));
}

#[test]
fn trailing_and_invalid_raw_deflate_bytes_are_rejected() {
    let mut trailing = HELLO_RAW_DEFLATE.to_vec();
    trailing.push(TRAILING_DEFLATE_BYTE);
    let trailing = FixtureEntry::deflated(PAYLOAD_PATH, &trailing, HELLO_SIZE, HELLO_CRC32);
    assert_rejected(&archive_with(trailing));

    let invalid = FixtureEntry::deflated(PAYLOAD_PATH, b"invalid", HELLO_SIZE, HELLO_CRC32);
    assert_rejected(&archive_with(invalid));
}

#[test]
fn stored_payload_crc_is_recomputed() {
    let mut entry = FixtureEntry::regular(b"Yolo.app/Contents/stored");
    entry.crc32 = EMPTY_CRC32.wrapping_add(1);
    assert_rejected(&archive_with(entry));
}

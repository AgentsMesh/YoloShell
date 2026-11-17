use super::*;

const TEST_INODE: u64 = 251;
const TEST_UUID_BYTE: u8 = 0x51;
const TEST_UUID_LENGTH: usize = 16;
const TEST_LENGTH: u64 = 7;
const TEST_TIMESTAMP: i64 = 1_700_000_000;
const DURABLE_IDENTITY_SCHEMA: u16 = 1;

pub(super) fn fixture() -> DurableFileVersion {
    let volume_uuid = [TEST_UUID_BYTE; TEST_UUID_LENGTH];
    let object = serde_json::from_value(serde_json::json!({
        "schema": DURABLE_IDENTITY_SCHEMA, "kind": yolo_durable_identity::ObjectKind::RegularFile,
        "birth": { "seconds": 0, "nanoseconds": 0 },
        "proof": { "macos": { "volume_uuid": volume_uuid, "file_id": TEST_INODE } }
    }))
    .unwrap();
    Record {
        schema: VERSION_SCHEMA,
        object,
        length: TEST_LENGTH,
        modified: FileTimestamp::new(TEST_TIMESTAMP, i64::default()),
        changed: FileTimestamp::new(TEST_TIMESTAMP, i64::default()),
    }
    .try_into()
    .unwrap()
}

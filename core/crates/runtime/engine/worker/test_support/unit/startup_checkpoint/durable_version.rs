use crate::paths::identity::DurableFileVersion;

const VERSION_SCHEMA: u16 = 1;
const OBJECT_SCHEMA: u16 = 1;
const BTRFS_FILESYSTEM_TYPE: u32 = 0x9123_683e;
const FILESYSTEM_UUID_BYTES: usize = 16;
const MAX_HANDLE_BYTES: usize = 128;
const MAX_TIMESTAMP_NANOSECONDS: i64 = 999_999_999;
pub(super) const SOURCE_OBJECT_FIELD: &str = "object";

pub(super) fn largest() -> DurableFileVersion {
    let timestamp = serde_json::json!({
        "seconds": i64::MIN,
        "nanoseconds": MAX_TIMESTAMP_NANOSECONDS,
    });
    serde_json::from_value(serde_json::json!({
        "schema": VERSION_SCHEMA,
        "object": {
            "schema": OBJECT_SCHEMA,
            "kind": super::constants::REGULAR_FILE_OBJECT_KIND,
            "birth": timestamp,
            "proof": { "linux": {
                "filesystem_type": BTRFS_FILESYSTEM_TYPE,
                "filesystem_uuid": vec![u8::MAX; FILESYSTEM_UUID_BYTES],
                "handle": { "kind": i32::MAX, "bytes": vec![u8::MAX; MAX_HANDLE_BYTES] },
            }},
        },
        "length": u64::MAX,
        "modified": timestamp,
        "changed": timestamp,
    }))
    .unwrap()
}

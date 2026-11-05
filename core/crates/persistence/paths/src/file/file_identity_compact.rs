use super::{FileIdentity, FileTimestamp};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Deserializer, Serializer};

const COMPONENT_BYTES: usize = std::mem::size_of::<u64>();
const IDENTITY_COMPONENTS: usize = 9;
const IDENTITY_BYTES: usize = COMPONENT_BYTES * IDENTITY_COMPONENTS;
const COMPACT_IDENTITY_ERROR: &str = "compact file identity is not canonical";
const TIMESTAMP_ERROR: &str = "compact file identity timestamp is invalid";

pub fn serialize<S>(value: &FileIdentity, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&URL_SAFE_NO_PAD.encode(encode(value)))
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<FileIdentity, D::Error>
where
    D: Deserializer<'de>,
{
    let encoded = String::deserialize(deserializer)?;
    let decoded = URL_SAFE_NO_PAD
        .decode(&encoded)
        .map_err(|_| serde::de::Error::custom(COMPACT_IDENTITY_ERROR))?;
    let bytes: [u8; IDENTITY_BYTES] = decoded
        .try_into()
        .map_err(|_| serde::de::Error::custom(COMPACT_IDENTITY_ERROR))?;
    if URL_SAFE_NO_PAD.encode(bytes) != encoded {
        return Err(serde::de::Error::custom(COMPACT_IDENTITY_ERROR));
    }
    decode(bytes).map_err(serde::de::Error::custom)
}

fn encode(value: &FileIdentity) -> [u8; IDENTITY_BYTES] {
    let components = [
        value.device.to_be_bytes(),
        value.inode.to_be_bytes(),
        value.length.to_be_bytes(),
        value.modified.seconds.to_be_bytes(),
        value.modified.nanoseconds.to_be_bytes(),
        value.changed.seconds.to_be_bytes(),
        value.changed.nanoseconds.to_be_bytes(),
        value.birth.seconds.to_be_bytes(),
        value.birth.nanoseconds.to_be_bytes(),
    ];
    let mut bytes = [0_u8; IDENTITY_BYTES];
    for (destination, component) in bytes.chunks_exact_mut(COMPONENT_BYTES).zip(components) {
        destination.copy_from_slice(&component);
    }
    bytes
}

fn decode(bytes: [u8; IDENTITY_BYTES]) -> Result<FileIdentity, &'static str> {
    let mut components = bytes.chunks_exact(COMPONENT_BYTES);
    let device = u64::from_be_bytes(next_component(&mut components));
    let inode = u64::from_be_bytes(next_component(&mut components));
    let length = u64::from_be_bytes(next_component(&mut components));
    let modified_seconds = i64::from_be_bytes(next_component(&mut components));
    let modified_nanoseconds = i64::from_be_bytes(next_component(&mut components));
    let changed_seconds = i64::from_be_bytes(next_component(&mut components));
    let changed_nanoseconds = i64::from_be_bytes(next_component(&mut components));
    let birth_seconds = i64::from_be_bytes(next_component(&mut components));
    let birth_nanoseconds = i64::from_be_bytes(next_component(&mut components));
    validate_nanoseconds([modified_nanoseconds, changed_nanoseconds, birth_nanoseconds])?;
    Ok(FileIdentity {
        device,
        inode,
        length,
        modified: FileTimestamp::new(modified_seconds, modified_nanoseconds),
        changed: FileTimestamp::new(changed_seconds, changed_nanoseconds),
        birth: FileTimestamp::new(birth_seconds, birth_nanoseconds),
    })
}

fn next_component(components: &mut std::slice::ChunksExact<'_, u8>) -> [u8; COMPONENT_BYTES] {
    components
        .next()
        .expect("fixed identity component count")
        .try_into()
        .expect("fixed identity component width")
}

fn validate_nanoseconds(values: [i64; 3]) -> Result<(), &'static str> {
    let valid = values.into_iter().all(|nanoseconds| {
        u64::try_from(nanoseconds)
            .is_ok_and(|value| value < crate::runtime_constants::NANOSECONDS_PER_SECOND)
    });
    valid.then_some(()).ok_or(TIMESTAMP_ERROR)
}

#[cfg(any(test, feature = "persistence-test-support"))]
#[path = "../../test_support/file_identity_compact.rs"]
mod test_support;
#[cfg(any(test, feature = "persistence-test-support"))]
pub use test_support::largest_for_test;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/file_identity/file_identity_compact_tests.rs"]
mod tests;

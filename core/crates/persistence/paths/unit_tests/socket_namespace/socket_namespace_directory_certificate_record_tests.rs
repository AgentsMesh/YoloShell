use super::*;
use crate::paths::identity::DurableObjectIdentity;
use yolo_durable_identity::ObjectKind;

const UUID_BYTES: usize = 16;
const UUID_BYTE: u8 = 5;
const INODE: u64 = 7;
const BIRTH_SECONDS: i64 = 11;
const BIRTH_NANOSECONDS: i64 = 13;
const OTHER_INODE: u64 = 17;
const FIELD_CHANGE: u32 = 1;
const DURABLE_IDENTITY_SCHEMA: u16 = 1;

fn object(kind: ObjectKind, inode: u64) -> DurableObjectIdentity {
    let uuid = [UUID_BYTE; UUID_BYTES];
    serde_json::from_value(serde_json::json!({
        "schema": DURABLE_IDENTITY_SCHEMA, "kind": kind,
        "birth": { "seconds": BIRTH_SECONDS, "nanoseconds": BIRTH_NANOSECONDS },
        "proof": { "macos": { "volume_uuid": uuid, "file_id": inode } }
    }))
    .unwrap()
}
fn directory() -> DurableObjectIdentity {
    object(ObjectKind::Directory, INODE)
}
fn valid() -> Certificate {
    Certificate::new(
        directory(),
        NamespaceBinding {
            schema_version: super::super::super::SOCKET_BINDING_SCHEMA_VERSION,
            effective_user_id: unsafe { libc::geteuid() },
            home_digest: "a".repeat(super::super::super::SOCKET_FULL_HASH_HEX_LENGTH),
        },
        object(ObjectKind::RegularFile, OTHER_INODE),
    )
    .unwrap()
}

#[test]
fn certificate_rejects_wrong_directory_schema_owner_and_digest() {
    let certificate = valid();
    certificate.validate(directory()).unwrap();
    let other = object(ObjectKind::Directory, OTHER_INODE);
    assert!(certificate.validate(other).is_err());
    let mut wrong_kind = valid();
    wrong_kind.registration = directory();
    assert!(wrong_kind.validate(directory()).is_err());
    let mut certificate = valid();
    certificate.schema_version = u16::MAX;
    assert!(certificate.validate(directory()).is_err());
    let mut certificate = valid();
    certificate.binding.schema_version += FIELD_CHANGE;
    assert!(certificate.validate(directory()).is_err());
    let mut certificate = valid();
    certificate.binding.effective_user_id = certificate
        .binding
        .effective_user_id
        .wrapping_add(FIELD_CHANGE);
    assert!(certificate.validate(directory()).is_err());
    for digest in [
        String::new(),
        "A".repeat(valid().binding.home_digest.len()),
        "z".repeat(valid().binding.home_digest.len()),
    ] {
        let mut certificate = valid();
        certificate.binding.home_digest = digest;
        assert!(certificate.validate(directory()).is_err());
    }
}

#[test]
fn certificate_budget_covers_two_maximum_bounded_native_proofs() {
    const HANDLE_BYTES: usize = 128;
    const BTRFS_MAGIC: u32 = 0x9123_683e;
    const MAX_NANOSECONDS: u32 = 999_999_999;
    let uuid = [u8::MAX; UUID_BYTES];
    let handle = vec![u8::MAX; HANDLE_BYTES];
    let identity = |kind| {
        serde_json::from_value(serde_json::json!({
            "schema": DURABLE_IDENTITY_SCHEMA, "kind": kind,
            "birth": {"seconds": i64::MIN, "nanoseconds": MAX_NANOSECONDS},
            "proof": {"linux": {"filesystem_type": BTRFS_MAGIC, "filesystem_uuid": uuid,
                "handle": {"kind": i32::MAX, "bytes": handle}}}
        }))
        .unwrap()
    };
    let certificate = Certificate::new(
        identity(ObjectKind::Directory),
        valid().binding,
        identity(ObjectKind::RegularFile),
    )
    .unwrap();
    assert!(serde_json::to_vec(&certificate).unwrap().len() <= ATTRIBUTE_MAX_BYTES);
}

#[test]
fn certificate_serialization_is_bounded_strict_and_round_trips() {
    let certificate = valid();
    let bytes = serde_json::to_vec(&certificate).unwrap();
    assert!(bytes.len() <= ATTRIBUTE_MAX_BYTES);
    assert_eq!(
        serde_json::from_slice::<Certificate>(&bytes).unwrap(),
        certificate
    );
    let mut value = serde_json::to_value(&certificate).unwrap();
    value["unexpected"] = true.into();
    assert!(serde_json::from_value::<Certificate>(value).is_err());
}

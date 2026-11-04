use super::*;
use crate::paths::{directory::PrivateDirectory, identity::DurableObjectIdentity};

const HOME: &str = "home";
const OTHER: &str = "other";
const RETIRED_SCHEMA: u16 = 1;
const UUID_BYTES: usize = 16;
const MAX_HANDLE_BYTES: usize = 128;
const BTRFS_MAGIC: u32 = 0x9123_683e;
const BTRFS_HANDLE_TYPE: i32 = 77;
const DURABLE_IDENTITY_SCHEMA: u16 = 1;
const MAX_BIRTH_NANOSECONDS: u32 = 999_999_999;
const SCHEMA_FIELD: &str = stringify!(schema_version);

fn binding() -> NamespaceBinding {
    NamespaceBinding {
        schema_version: super::super::SOCKET_BINDING_SCHEMA_VERSION,
        effective_user_id: unsafe { libc::geteuid() },
        home_digest: "a".repeat(super::super::SOCKET_FULL_HASH_HEX_LENGTH),
    }
}

#[test]
fn valid_foreign_durable_identity_and_retired_record_are_rejected_without_repair() {
    for retired in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let home = PrivateDirectory::ensure(&root.path().join(HOME)).unwrap();
        let other = PrivateDirectory::ensure(&root.path().join(OTHER)).unwrap();
        drop(retain(home.path(), &binding(), true).unwrap());
        let path = home.path().join(REGISTRATION_FILE);
        let mut wire: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        if retired {
            wire[SCHEMA_FIELD] = serde_json::json!(RETIRED_SCHEMA);
        } else {
            wire["home"] = serde_json::to_value(other.durable_identity().unwrap()).unwrap();
        }
        let bytes = serde_json::to_vec(&wire).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        assert!(inspect(home.path(), &binding()).is_err());
        assert!(retain(home.path(), &binding(), true).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn current_registration_budget_covers_maximum_bounded_identity_pretty_json() {
    let uuid = [u8::MAX; UUID_BYTES];
    let handle = vec![u8::MAX; MAX_HANDLE_BYTES];
    let home: DurableObjectIdentity = serde_json::from_value(serde_json::json!({
        "schema": DURABLE_IDENTITY_SCHEMA, "kind": "directory",
        "birth": {"seconds": i64::MIN, "nanoseconds": MAX_BIRTH_NANOSECONDS},
        "proof": {"linux": {"filesystem_type": BTRFS_MAGIC, "filesystem_uuid": uuid,
            "handle": {"kind": BTRFS_HANDLE_TYPE, "bytes": handle}}}
    }))
    .unwrap();
    let record = record::Record::new(home, &binding()).unwrap();
    let bytes = serde_json::to_vec_pretty(&record).unwrap();
    assert!(bytes.len() as u64 <= REGISTRATION_MAX_BYTES);
    serde_json::from_slice::<record::Record>(&bytes)
        .unwrap()
        .validate(home, &binding())
        .unwrap();
}

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tempfile::tempdir;
use uuid::Uuid;
use yolo_storage::{
    JsonLimits, SessionNamespace, StorageError, atomic_write_json, decode_json, encode_json,
    read_json,
};

const SESSION: Uuid = Uuid::from_u128(1);
const VALUE: &str = "value";
const SMALL_LIMIT: usize = 4;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Record {
    value: String,
}

#[test]
fn namespace_rejects_relative_and_nil_paths() {
    assert!(matches!(
        SessionNamespace::new("relative"),
        Err(StorageError::InvalidPath)
    ));
    let namespace = SessionNamespace::new("/tmp/yolo-storage-test").unwrap();
    assert!(matches!(
        namespace.session_dir(Uuid::nil()),
        Err(StorageError::NilSession)
    ));
    assert!(
        namespace
            .manifest(SESSION)
            .unwrap()
            .ends_with("manifest.json")
    );
}

#[test]
fn json_encoding_is_bounded_and_round_trips() {
    let record = Record {
        value: VALUE.into(),
    };
    let bytes = encode_json(&record, JsonLimits::default()).unwrap();
    assert_eq!(
        decode_json::<Record>(&bytes, JsonLimits::default()).unwrap(),
        record
    );
    assert!(matches!(
        encode_json(
            &record,
            JsonLimits {
                max_bytes: SMALL_LIMIT
            }
        ),
        Err(StorageError::Limit)
    ));
    assert!(matches!(
        decode_json::<Record>(
            &bytes,
            JsonLimits {
                max_bytes: SMALL_LIMIT
            }
        ),
        Err(StorageError::Limit)
    ));
}

#[test]
fn atomic_write_replaces_value_and_leaves_private_file() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("state.json");
    let first = Record {
        value: "first".into(),
    };
    let second = Record {
        value: "second".into(),
    };
    atomic_write_json(&path, &first, JsonLimits::default()).unwrap();
    atomic_write_json(&path, &second, JsonLimits::default()).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(
        decode_json::<Record>(&bytes, JsonLimits::default()).unwrap(),
        second
    );
    assert!(
        !PathBuf::from(&path)
            .with_file_name(".yolo-storage-state.json")
            .exists()
    );
    assert_eq!(
        read_json::<Record>(&path, JsonLimits::default()).unwrap(),
        second
    );
    assert!(matches!(
        atomic_write_json(
            PathBuf::from("relative.json").as_path(),
            &first,
            JsonLimits::default()
        ),
        Err(StorageError::InvalidPath)
    ));
    assert!(matches!(
        atomic_write_json(
            PathBuf::from("/tmp//state.json").as_path(),
            &first,
            JsonLimits::default()
        ),
        Err(StorageError::InvalidPath)
    ));
}

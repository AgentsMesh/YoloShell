use crate::{
    DurableObjectIdentity, Error, ObjectKind,
    model::{Birth, HANDLE_CAPACITY, Handle, Proof, UUID_BYTES},
};

#[path = "../test_support/model.rs"]
mod fixture;
mod handles;
mod streaming;
mod wire;

const INVALID_NANOSECONDS: u32 = 1_000_000_000;
const PRE_EPOCH_SECONDS: i64 = -1;

#[test]
fn identity_accepts_available_epoch_zero_and_pre_epoch_birth() {
    for seconds in [i64::default(), PRE_EPOCH_SECONDS] {
        let value = DurableObjectIdentity::new(
            ObjectKind::Directory,
            Birth {
                seconds,
                nanoseconds: u32::default(),
            },
            fixture::linux().record.proof,
        )
        .unwrap();
        assert_eq!(value.kind(), ObjectKind::Directory);
    }
}

#[test]
fn invalid_birth_uuid_and_native_object_fields_are_rejected() {
    let mut record = fixture::linux().record;
    record.birth.nanoseconds = INVALID_NANOSECONDS;
    assert!(DurableObjectIdentity::try_from(record).is_err());
    for mut record in [fixture::linux().record, fixture::macos().record] {
        match &mut record.proof {
            Proof::Linux {
                filesystem_uuid, ..
            } => *filesystem_uuid = [u8::default(); UUID_BYTES],
            Proof::Macos { volume_uuid, .. } => *volume_uuid = [u8::default(); UUID_BYTES],
        }
        assert!(DurableObjectIdentity::try_from(record).is_err());
    }
    let mut record = fixture::macos().record;
    if let Proof::Macos { file_id, .. } = &mut record.proof {
        *file_id = u64::default();
    }
    assert!(DurableObjectIdentity::try_from(record).is_err());
    let mut record = fixture::linux().record;
    if let Proof::Linux {
        filesystem_type, ..
    } = &mut record.proof
    {
        *filesystem_type = u32::default();
    }
    assert!(DurableObjectIdentity::try_from(record).is_err());
}

#[test]
fn error_classification_keeps_unsupported_changed_and_io_distinct() {
    assert!(Error::Changed.to_string().contains("changed"));
    assert!(
        Error::Unsupported("test filesystem")
            .to_string()
            .contains("unsupported")
    );
    let source = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
    let error = Error::Io {
        operation: "test operation",
        source,
    };
    assert!(std::error::Error::source(&error).is_some());
}

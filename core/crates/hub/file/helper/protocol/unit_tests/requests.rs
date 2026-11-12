use crate::fixtures::*;
use yolo_hub_file_helper_protocol::*;
use yolo_hub_file_protocol::{
    FileRootSummary, MAX_FILE_PATH_BYTES, MAX_FILE_ROOTS, ReadFileRequest,
};
use yolo_hub_identity::HubId;
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn exact_inspection_and_read_documents_validate_and_roundtrip() {
    for fields in [inspect(), read(), build()] {
        let request = Request::new(fields.clone()).unwrap();
        assert_eq!(request.fields(), &fields);
        assert_eq!(RequestFields::from(request.clone()), fields);
        assert_eq!(Request::try_from(fields).unwrap(), request);
        roundtrip(&request);
        let mut object = serde_json::to_value(&request).unwrap();
        object[EXTRA_FIELD] = true.into();
        assert!(serde_json::from_value::<Request>(object).is_err());
        let mut object = serde_json::to_value(request).unwrap();
        object[SCHEMA_FIELD] = u16::MAX.into();
        assert!(serde_json::from_value::<Request>(object).is_err());
    }
}

#[test]
fn malformed_envelopes_and_protection_bounds_are_rejected_before_execution() {
    let mut fields = inspect();
    fields.request_id = uuid::Uuid::nil();
    assert_eq!(Request::new(fields), Err(PublicError::InvalidRequest));
    let mut fields = inspect();
    fields.protected.clear();
    assert_eq!(Request::new(fields), Err(PublicError::InvalidRequest));
    let mut fields = inspect();
    fields.protected = vec![PROTECTED_PATH.into(); MAX_PROTECTED_PATHS + 1];
    assert_eq!(Request::new(fields), Err(PublicError::InvalidRequest));
    for path in [
        "relative".into(),
        "/".into(),
        "/a/../b".into(),
        format!("/{}", REPEAT_CHARACTER.repeat(MAX_FILE_PATH_BYTES)),
    ] {
        let mut fields = inspect();
        fields.protected = vec![path];
        assert_eq!(Request::new(fields), Err(PublicError::InvalidMetadata));
    }
    let mut fields = inspect();
    fields.protected = vec![PROTECTED_PATH.into(); MAX_PROTECTED_PATHS];
    assert!(Request::new(fields).is_ok());
}

#[test]
fn inspection_is_one_origin_and_cannot_repeat_or_overflow_roots() {
    let root = root();
    let mut fields = inspect();
    fields.origin = root.fields().target.address.hub_id;
    fields.command = Command::Inspect {
        roots: vec![root.clone(); MAX_FILE_ROOTS + 1],
    };
    assert_eq!(
        Request::new(fields.clone()),
        Err(PublicError::InvalidRequest)
    );
    fields.command = Command::Inspect {
        roots: vec![root.clone(), root.clone()],
    };
    assert_eq!(
        Request::new(fields.clone()),
        Err(PublicError::InvalidRequest)
    );
    fields.command = Command::Inspect { roots: vec![root] };
    fields.origin = HubId::new();
    assert_eq!(
        Request::new(fields.clone()),
        Err(PublicError::OriginMismatch)
    );
    fields.command = Command::Inspect { roots: vec![] };
    assert!(Request::new(fields).is_ok());
}

#[test]
fn reads_must_preserve_both_the_origin_and_the_approved_root_context() {
    let mut fields = read();
    fields.origin = HubId::new();
    assert_eq!(Request::new(fields), Err(PublicError::OriginMismatch));
    let mut fields = read();
    if let Command::Read { binding, request } = &mut fields.command {
        let mut other = binding.root.fields().clone();
        other.target.epoch = yolo_hub_identity::FileRootEpoch::new();
        binding.root = FileRootSummary::new(other).unwrap();
        let mut stale = request.fields().clone();
        stale.target.address.root_id = yolo_hub_identity::FileRootId::new();
        *request = ReadFileRequest::new(stale).unwrap();
    }
    assert_eq!(Request::new(fields), Err(PublicError::TargetMismatch));
}

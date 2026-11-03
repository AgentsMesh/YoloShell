use crate::fixtures::*;
use yolo_hub_file_protocol::*;
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn requests_roundtrip_and_require_an_origin_version_for_nonzero_offsets() {
    let fields = request();
    let first = ReadFileRequest::new(fields.clone()).unwrap();
    assert_eq!(first.fields(), &fields);
    assert_eq!(ReadFileFields::from(first.clone()), fields);
    assert_eq!(ReadFileRequest::try_from(fields.clone()).unwrap(), first);
    roundtrip(&first);
    let mut next = fields;
    next.offset = PAYLOAD.len() as u64;
    assert_eq!(
        ReadFileRequest::new(next.clone()),
        Err(PublicError::InvalidRequest)
    );
    next.expected_version = Some(VERSION);
    let next = ReadFileRequest::new(next).unwrap();
    roundtrip(&next);
    let mut object = serde_json::to_value(next).unwrap();
    object[VERSION_FIELD] = serde_json::Value::Null;
    assert!(serde_json::from_value::<ReadFileRequest>(object).is_err());
}

#[test]
fn invalid_bounds_and_schema_cannot_reach_origin_io() {
    for length in [0, MAX_FILE_READ_BYTES + 1, u32::MAX] {
        let mut fields = request();
        fields.length = length;
        assert_eq!(
            ReadFileRequest::new(fields),
            Err(PublicError::InvalidRequest)
        );
    }
    for offset in [MAX_FILE_OFFSET, u64::MAX] {
        let mut fields = request();
        fields.offset = offset;
        fields.expected_version = Some(VERSION);
        assert_eq!(
            ReadFileRequest::new(fields),
            Err(PublicError::InvalidRequest)
        );
    }
    let mut fields = request();
    fields.schema_version = u16::MAX;
    assert_eq!(
        ReadFileRequest::new(fields),
        Err(PublicError::UnsupportedSchema)
    );
    let mut fields = request();
    fields.length = MAX_FILE_READ_BYTES;
    assert!(ReadFileRequest::new(fields).is_ok());
    let mut fields = request();
    fields.offset = MAX_FILE_OFFSET - u64::from(fields.length);
    fields.expected_version = Some(VERSION);
    assert!(ReadFileRequest::new(fields).is_ok());
    let mut object = serde_json::to_value(ReadFileRequest::new(request()).unwrap()).unwrap();
    object[SCHEMA_FIELD] = u16::MAX.into();
    assert!(serde_json::from_value::<ReadFileRequest>(object.clone()).is_err());
    object[SCHEMA_FIELD] = FILE_PROTOCOL_SCHEMA_VERSION.into();
    object[EXTRA_FIELD] = true.into();
    assert!(serde_json::from_value::<ReadFileRequest>(object).is_err());
}

use crate::fixtures::*;
use yolo_hub_file_protocol::*;
use yolo_hub_identity::FileRootEpoch;
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn exact_results_roundtrip_and_never_print_contents_in_debug_logs() {
    let request = ReadFileRequest::new(request()).unwrap();
    let fields = result(&request);
    let result = ReadFileResult::new(fields.clone()).unwrap();
    assert!(result.fields() == &fields);
    assert!(ReadFileResultFields::from(result.clone()) == fields);
    assert_eq!(ReadFileResult::try_from(fields).unwrap(), result);
    result.validate_for(&request).unwrap();
    roundtrip(&result);
    let debug = format!("{result:?}");
    assert!(!debug.contains(std::str::from_utf8(PAYLOAD).unwrap()));
    let mut wire = serde_json::to_value(result).unwrap();
    wire[EXTRA_FIELD] = true.into();
    assert!(serde_json::from_value::<ReadFileResult>(wire).is_err());
}

#[test]
fn results_reject_overflows_unbounded_bytes_inconsistent_eof_and_no_progress() {
    let request = ReadFileRequest::new(request()).unwrap();
    let mut fields = result(&request);
    fields.schema_version = u16::MAX;
    assert_eq!(
        ReadFileResult::new(fields),
        Err(PublicError::UnsupportedSchema)
    );
    let mut fields = result(&request);
    fields.offset = u64::MAX;
    assert_eq!(
        ReadFileResult::new(fields),
        Err(PublicError::InvalidPayload)
    );
    let mut fields = result(&request);
    fields.data = vec![0; MAX_FILE_READ_BYTES as usize + 1];
    fields.total_length = fields.data.len() as u64;
    assert_eq!(
        ReadFileResult::new(fields),
        Err(PublicError::InvalidPayload)
    );
    let mut fields = result(&request);
    fields.total_length = MAX_FILE_OFFSET + 1;
    fields.eof = false;
    assert_eq!(
        ReadFileResult::new(fields),
        Err(PublicError::InvalidPayload)
    );
    let mut fields = result(&request);
    fields.total_length = 0;
    assert_eq!(
        ReadFileResult::new(fields),
        Err(PublicError::InvalidPayload)
    );
    let mut fields = result(&request);
    fields.eof = false;
    assert_eq!(
        ReadFileResult::new(fields),
        Err(PublicError::InvalidPayload)
    );
    let mut fields = result(&request);
    fields.data.clear();
    fields.eof = false;
    assert_eq!(
        ReadFileResult::new(fields),
        Err(PublicError::InvalidPayload)
    );
    let mut fields = result(&request);
    fields.data.clear();
    fields.total_length = 0;
    ReadFileResult::new(fields)
        .unwrap()
        .validate_for(&request)
        .unwrap();
}

#[test]
fn foreign_targets_paths_offsets_versions_and_short_chunks_are_not_valid_receipts() {
    let request = ReadFileRequest::new(request()).unwrap();
    let mut fields = result(&request);
    fields.target.epoch = FileRootEpoch::new();
    assert_eq!(
        ReadFileResult::new(fields).unwrap().validate_for(&request),
        Err(PublicError::TargetMismatch)
    );
    let mut fields = result(&request);
    fields.relative_path = RelativeFilePath::new(OTHER_RELATIVE.into()).unwrap();
    assert_eq!(
        ReadFileResult::new(fields).unwrap().validate_for(&request),
        Err(PublicError::TargetMismatch)
    );
    let mut fields = result(&request);
    fields.offset = 1;
    fields.total_length += 1;
    assert_eq!(
        ReadFileResult::new(fields).unwrap().validate_for(&request),
        Err(PublicError::TargetMismatch)
    );
    let mut fields = request.fields().clone();
    fields.expected_version = Some(OTHER_VERSION);
    let pinned = ReadFileRequest::new(fields.clone()).unwrap();
    assert_eq!(
        ReadFileResult::new(result(&request))
            .unwrap()
            .validate_for(&pinned),
        Err(PublicError::TargetMismatch)
    );
    fields.expected_version = Some(VERSION);
    let pinned = ReadFileRequest::new(fields).unwrap();
    ReadFileResult::new(result(&request))
        .unwrap()
        .validate_for(&pinned)
        .unwrap();
    let mut fields = result(&request);
    fields.data.pop();
    fields.eof = false;
    assert_eq!(
        ReadFileResult::new(fields).unwrap().validate_for(&request),
        Err(PublicError::InvalidPayload)
    );
}

use crate::fixture::*;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use yolo_hub_file_protocol::*;
use yolo_hub_file_reader::ReadRoot;
use yolo_hub_identity::FileRootEpoch;
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn chunks_preserve_one_version_and_do_not_modify_project_permissions() {
    let fixture = Fixture::new();
    fs::set_permissions(&fixture.workspace, fs::Permissions::from_mode(PROJECT_MODE)).unwrap();
    let before = fs::metadata(&fixture.workspace).unwrap().mode();
    let root = fixture.root();
    assert_eq!(root.summary(), &fixture.summary);
    let request = fixture.request(FILE_PATH);
    let first = root.read(&request).unwrap();
    assert_eq!(first.fields().data, PAYLOAD[..READ_LENGTH as usize]);
    assert!(!first.fields().eof);
    let mut combined = first.fields().data.clone();
    while combined.len() < PAYLOAD.len() {
        let mut fields = request.fields().clone();
        fields.offset = combined.len() as u64;
        fields.expected_version = Some(first.fields().version);
        let next = ReadFileRequest::new(fields).unwrap();
        let response = root.prepare(&next).unwrap().read().unwrap();
        response.validate_for(&next).unwrap();
        assert_eq!(response.fields().version, first.fields().version);
        combined.extend_from_slice(&response.fields().data);
        assert_eq!(response.fields().eof, combined.len() == PAYLOAD.len());
    }
    assert_eq!(combined, PAYLOAD);
    assert_eq!(fs::metadata(&fixture.workspace).unwrap().mode(), before);
}

#[test]
fn empty_eof_and_out_of_range_are_distinct_from_a_short_unconfirmed_read() {
    let fixture = Fixture::new();
    fs::write(fixture.workspace.join(FILE_PATH), []).unwrap();
    let root = fixture.root();
    let request = fixture.request(FILE_PATH);
    let response = root.read(&request).unwrap();
    assert!(response.fields().eof);
    assert!(response.fields().data.is_empty());
    let mut fields = request.fields().clone();
    fields.offset = u64::from(SINGLE_BYTE);
    fields.expected_version = Some(response.fields().version);
    assert_eq!(
        root.read(&ReadFileRequest::new(fields).unwrap()),
        Err(PublicError::InvalidRequest)
    );
}

#[test]
fn file_versions_do_not_transfer_to_another_root_epoch_or_changed_file() {
    let fixture = Fixture::new();
    let root = fixture.root();
    let request = fixture.request(FILE_PATH);
    let first = root.read(&request).unwrap();
    let mut changed_target = fixture.summary.fields().clone();
    changed_target.target.epoch = FileRootEpoch::new();
    let second = ReadRoot::open(
        FileRootSummary::new(changed_target.clone()).unwrap(),
        &[fixture.private.clone()],
    )
    .unwrap();
    assert_eq!(second.read(&request), Err(PublicError::TargetMismatch));
    let mut continuation = request.fields().clone();
    continuation.target = changed_target.target;
    continuation.expected_version = Some(first.fields().version);
    assert_eq!(
        second.read(&ReadFileRequest::new(continuation).unwrap()),
        Err(PublicError::Conflict)
    );
    fs::write(fixture.workspace.join(FILE_PATH), CHANGED).unwrap();
    let mut continuation = request.fields().clone();
    continuation.offset = u64::from(READ_LENGTH);
    continuation.expected_version = Some(first.fields().version);
    assert_eq!(
        root.read(&ReadFileRequest::new(continuation).unwrap()),
        Err(PublicError::Conflict)
    );
}

use crate::constants::*;
use yolo_hub_file_protocol::*;
use yolo_hub_file_reader_test_support::{FILE_PATH, Fixture, READ_LENGTH};
use yolo_hub_file_service::FileService;

pub fn configuration(fixture: &Fixture) -> FileRootConfiguration {
    let root = fixture.summary.fields();
    FileRootConfiguration::new(FileRootFields {
        root_id: root.target.address.root_id,
        name: root.name.clone(),
        path: root.path.clone(),
    })
    .unwrap()
}
pub async fn service(
    fixture: &Fixture,
    roots: &[FileRootConfiguration],
) -> Result<FileService, yolo_hub_terminal_protocol::PublicError> {
    let program = std::path::PathBuf::from(std::env::var(HELPER_ENV).unwrap())
        .canonicalize()
        .unwrap();
    FileService::prepare(
        fixture.summary.fields().target.address.hub_id,
        roots,
        vec![fixture.private.clone()],
        Some(program),
    )
    .await
}
pub fn request(root: &FileRootSummary) -> ReadFileRequest {
    ReadFileRequest::new(ReadFileFields {
        schema_version: FILE_PROTOCOL_SCHEMA_VERSION,
        target: root.fields().target,
        relative_path: RelativeFilePath::new(FILE_PATH.into()).unwrap(),
        offset: 0,
        length: READ_LENGTH,
        expected_version: None,
    })
    .unwrap()
}

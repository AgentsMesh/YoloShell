use crate::fixtures::{constants::*, *};
use yolo_hub_file_protocol::*;
use yolo_hub_identity::{FileRootEpoch, FileRootId};

pub fn file_root() -> FileRootSummary {
    FileRootSummary::new(FileRootSummaryFields {
        target: FileRootTarget {
            address: FileRootAddress {
                hub_id: hub(HUB_ID),
                root_id: FileRootId::from_uuid(id(FILE_ROOT_ID)).unwrap(),
            },
            epoch: FileRootEpoch::from_uuid(id(FILE_ROOT_EPOCH)).unwrap(),
        },
        name: FILE_ROOT_NAME.into(),
        path: DIRECTORY.into(),
    })
    .unwrap()
}
pub fn file_read() -> ReadFileRequest {
    ReadFileRequest::new(ReadFileFields {
        schema_version: FILE_PROTOCOL_SCHEMA_VERSION,
        target: file_root().fields().target,
        relative_path: RelativeFilePath::new(FILE_RELATIVE.into()).unwrap(),
        offset: 0,
        length: FILE_CONTENTS.len() as u32,
        expected_version: None,
    })
    .unwrap()
}
pub fn file_reply() -> ReadFileResult {
    ReadFileResult::new(ReadFileResultFields {
        schema_version: FILE_PROTOCOL_SCHEMA_VERSION,
        target: file_root().fields().target,
        relative_path: file_read().fields().relative_path.clone(),
        offset: 0,
        version: yolo_crypto::Sha256Digest::repeated(FILE_VERSION),
        total_length: FILE_CONTENTS.len() as u64,
        data: FILE_CONTENTS.to_vec(),
        eof: true,
    })
    .unwrap()
}

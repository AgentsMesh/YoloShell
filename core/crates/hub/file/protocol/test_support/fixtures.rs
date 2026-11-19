pub use crate::constants::*;
use yolo_hub_file_protocol::*;
use yolo_hub_identity::{FileRootEpoch, FileRootId, HubId};

pub fn target() -> FileRootTarget {
    FileRootTarget {
        address: FileRootAddress {
            hub_id: HubId::new(),
            root_id: FileRootId::new(),
        },
        epoch: FileRootEpoch::new(),
    }
}
pub fn root() -> FileRootFields {
    FileRootFields {
        root_id: FileRootId::new(),
        name: ROOT_NAME.into(),
        path: ROOT_PATH.into(),
    }
}
pub fn request() -> ReadFileFields {
    ReadFileFields {
        schema_version: FILE_PROTOCOL_SCHEMA_VERSION,
        target: target(),
        relative_path: RelativeFilePath::new(RELATIVE.into()).unwrap(),
        offset: 0,
        length: PAYLOAD.len() as u32,
        expected_version: None,
    }
}
pub fn result(request: &ReadFileRequest) -> ReadFileResultFields {
    ReadFileResultFields {
        schema_version: FILE_PROTOCOL_SCHEMA_VERSION,
        target: request.fields().target,
        relative_path: request.fields().relative_path.clone(),
        offset: request.fields().offset,
        version: VERSION,
        total_length: PAYLOAD.len() as u64,
        data: PAYLOAD.to_vec(),
        eof: true,
    }
}
pub fn roundtrip<T>(value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    assert_eq!(
        &serde_json::from_slice::<T>(&serde_json::to_vec(value).unwrap()).unwrap(),
        value
    );
}

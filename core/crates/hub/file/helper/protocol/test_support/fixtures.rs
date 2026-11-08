pub use crate::constants::*;
use yolo_crypto::Sha256Digest;
use yolo_hub_file_helper_protocol::*;
use yolo_hub_file_protocol::*;
use yolo_hub_identity::{FileRootEpoch, FileRootId, HubId};

pub fn root() -> FileRootSummary {
    FileRootSummary::new(FileRootSummaryFields {
        target: FileRootTarget {
            address: FileRootAddress {
                hub_id: HubId::new(),
                root_id: FileRootId::new(),
            },
            epoch: FileRootEpoch::new(),
        },
        name: ROOT_NAME.into(),
        path: ROOT_PATH.into(),
    })
    .unwrap()
}
pub fn binding(root: FileRootSummary) -> RootBinding {
    RootBinding {
        root,
        fingerprint: Sha256Digest::repeated(1),
    }
}
pub fn inspect() -> RequestFields {
    let root = root();
    RequestFields {
        schema_version: FILE_HELPER_SCHEMA,
        request_id: uuid::Uuid::new_v4(),
        origin: root.fields().target.address.hub_id,
        protected: vec![PROTECTED_PATH.into()],
        command: Command::Inspect { roots: vec![root] },
    }
}
pub fn read() -> RequestFields {
    let root = root();
    let request = ReadFileRequest::new(ReadFileFields {
        schema_version: FILE_PROTOCOL_SCHEMA_VERSION,
        target: root.fields().target,
        relative_path: RelativeFilePath::new(FILE_PATH.into()).unwrap(),
        offset: 0,
        length: PAYLOAD.len() as u32,
        expected_version: None,
    })
    .unwrap();
    RequestFields {
        schema_version: FILE_HELPER_SCHEMA,
        request_id: uuid::Uuid::new_v4(),
        origin: root.fields().target.address.hub_id,
        protected: vec![PROTECTED_PATH.into()],
        command: Command::Read {
            binding: binding(root),
            request,
        },
    }
}
pub fn payload(request: &Request) -> Payload {
    match &request.fields().command {
        Command::Build {} => Payload::Build {
            image: yolo_build_image::BuildImage::new(
                REPEAT_CHARACTER.repeat(yolo_build_image::SHA256_HEX_BYTES),
                BUILD_BYTES,
            )
            .unwrap(),
        },
        Command::Inspect { roots } => Payload::Inspected {
            bindings: roots.iter().cloned().map(binding).collect(),
        },
        Command::Read { request, .. } => Payload::Read {
            file: ReadFileResult::new(ReadFileResultFields {
                schema_version: FILE_PROTOCOL_SCHEMA_VERSION,
                target: request.fields().target,
                relative_path: request.fields().relative_path.clone(),
                offset: 0,
                version: Sha256Digest::repeated(1),
                total_length: PAYLOAD.len() as u64,
                data: PAYLOAD.to_vec(),
                eof: true,
            })
            .unwrap(),
        },
    }
}
pub fn build() -> RequestFields {
    let mut fields = inspect();
    fields.protected.clear();
    fields.command = Command::Build {};
    fields
}
pub fn roundtrip<
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
>(
    value: &T,
) {
    assert_eq!(
        &serde_json::from_slice::<T>(&serde_json::to_vec(value).unwrap()).unwrap(),
        value
    );
    assert_eq!(
        &yolo_capnp_codec::decode::<T>(&yolo_capnp_codec::encode(value).unwrap()).unwrap(),
        value
    );
}

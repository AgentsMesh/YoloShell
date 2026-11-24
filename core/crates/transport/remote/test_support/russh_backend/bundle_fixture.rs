use super::{
    constants::*,
    deployment_support::artifact,
    fixture::{MemoryConnection, connect_memory_with_fs},
    memory_fs::MemoryFilesystem,
};
use crate::remote::{BundlePayloads, BundleStageRequest};
use std::{path::PathBuf, sync::Arc};
use yolo_distribution_bundle::{
    Architecture, ArtifactDescriptor, BundleArtifacts, BundleDescriptor, Platform, Target,
};

pub const ROOT: &str = "/srv/yolo-bundle";
pub const PARENT: &str = "/srv";
pub const PARENT_MODE: u32 = 0o755;
pub const SOURCE_REVISION: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub const SELECTION_BYTES: &[u8] = b"{\"selection\":\"test\"}";
pub const EXTRA_BYTE: usize = 1;

pub async fn fixture() -> MemoryConnection {
    let filesystem = MemoryFilesystem::default();
    filesystem.insert_directory(PARENT, PARENT_MODE);
    connect_memory_with_fs(false, filesystem).await.unwrap()
}

pub fn request() -> BundleStageRequest {
    let binary = artifact(INITIAL_RUNTIME_VERSION, INITIAL_BINARY_FILL);
    let bytes: Arc<[u8]> = binary.bytes().into();
    let entry = ArtifactDescriptor::new(binary.sha256(), bytes.len() as u64).unwrap();
    let descriptor = BundleDescriptor::new(
        SOURCE_REVISION,
        INITIAL_RUNTIME_VERSION,
        Target {
            platform: Platform::Linux,
            architecture: Architecture::X86_64,
        },
        BundleArtifacts { runtime: entry },
    )
    .unwrap();
    BundleStageRequest::new(
        PathBuf::from(ROOT),
        uuid::Uuid::new_v4(),
        descriptor,
        BundlePayloads { runtime: bytes },
    )
    .unwrap()
}

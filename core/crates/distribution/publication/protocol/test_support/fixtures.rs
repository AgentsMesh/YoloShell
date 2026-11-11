use super::constants::*;
use uuid::Uuid;
use yolo_distribution_bundle::*;
use yolo_distribution_publication_protocol::*;

pub fn descriptor(version: &str) -> BundleDescriptor {
    let artifact = ArtifactDescriptor::new(DIGEST, ARTIFACT_BYTES).unwrap();
    BundleDescriptor::new(
        REVISION,
        version,
        Target {
            platform: Platform::Linux,
            architecture: Architecture::X86_64,
        },
        BundleArtifacts { runtime: artifact },
    )
    .unwrap()
}
pub fn request() -> PublicationRequest {
    PublicationRequest::new(
        ROOT,
        Uuid::new_v4(),
        Uuid::new_v4(),
        descriptor(VERSION),
        None,
    )
    .unwrap()
}

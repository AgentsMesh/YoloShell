use super::constants::*;
use yolo_distribution_bundle::*;

pub fn target() -> Target {
    Target {
        platform: Platform::Linux,
        architecture: Architecture::X86_64,
    }
}

pub fn artifacts() -> BundleArtifacts {
    BundleArtifacts {
        runtime: ArtifactDescriptor::new(RUNTIME_DIGEST, RUNTIME_BYTES).unwrap(),
    }
}

pub fn descriptor() -> BundleDescriptor {
    BundleDescriptor::new(REVISION, VERSION, target(), artifacts()).unwrap()
}

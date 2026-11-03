use yolo_distribution_bundle::{
    Architecture, ArtifactDescriptor, ArtifactRole, BundleArtifacts, BundleDescriptor, Platform,
    Target,
};
use yolo_release_contracts::*;
pub fn descriptor(revision: &str, architecture: &str) -> BundleDescriptor {
    let architecture = if architecture == LINUX_AMD64_RUNTIME_ARCHITECTURE {
        Architecture::X86_64
    } else {
        Architecture::Aarch64
    };
    let [runtime] = ArtifactRole::ALL.map(|role| {
        let bytes = format!("{}:{}", architecture.as_str(), role.executable_name());
        ArtifactDescriptor::new(
            yolo_release_foundation::digest::bytes_sha256(bytes.as_bytes()),
            bytes.len() as u64,
        )
        .unwrap()
    });
    BundleDescriptor::new(
        revision,
        LINUX_RUNTIME_VERSION,
        Target {
            platform: Platform::Linux,
            architecture,
        },
        BundleArtifacts { runtime },
    )
    .unwrap()
}

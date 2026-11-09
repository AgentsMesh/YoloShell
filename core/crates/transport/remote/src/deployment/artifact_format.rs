use super::artifact::{BinaryTarget, RemoteArchitecture, RemotePlatform};

pub(super) fn binary_target(bytes: &[u8]) -> Option<BinaryTarget> {
    let target = yolo_distribution_bundle::binary_target(bytes)?;
    Some(BinaryTarget {
        platform: match target.platform {
            yolo_distribution_bundle::Platform::Linux => RemotePlatform::Linux,
            yolo_distribution_bundle::Platform::MacOs => RemotePlatform::MacOs,
        },
        architecture: match target.architecture {
            yolo_distribution_bundle::Architecture::Aarch64 => RemoteArchitecture::Aarch64,
            yolo_distribution_bundle::Architecture::X86_64 => RemoteArchitecture::X86_64,
        },
    })
}

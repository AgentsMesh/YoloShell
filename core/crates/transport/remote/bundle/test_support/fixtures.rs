use super::constants::*;
use uuid::Uuid;
use yolo_distribution_bundle::*;
use yolo_remote::{BundlePayloads, BundleStageRequest};

pub fn stage() -> BundleStageRequest {
    let payloads = ArtifactRole::ALL.map(|role| {
        let mut bytes = vec![0; ELF_BYTES];
        bytes[..ELF_MAGIC.len()].copy_from_slice(ELF_MAGIC);
        bytes[ELF_CLASS_OFFSET] = ELF_64;
        bytes[ELF_ENDIAN_OFFSET] = ELF_LITTLE_ENDIAN;
        bytes[ELF_VERSION_OFFSET] = ELF_CURRENT;
        bytes[ELF_MACHINE_OFFSET..].copy_from_slice(&ELF_X86.to_le_bytes());
        bytes.extend_from_slice(role.as_str().as_bytes());
        bytes
    });
    let [runtime] = payloads.each_ref().map(|bytes| {
        ArtifactDescriptor::new(yolo_crypto::sha256_hex(bytes), bytes.len() as u64).unwrap()
    });
    let descriptor = BundleDescriptor::new(
        REVISION,
        VERSION,
        Target {
            platform: Platform::Linux,
            architecture: Architecture::X86_64,
        },
        BundleArtifacts { runtime },
    )
    .unwrap();
    let [runtime] = payloads.map(Into::into);
    BundleStageRequest::new(
        ROOT.into(),
        Uuid::new_v4(),
        descriptor,
        BundlePayloads { runtime },
    )
    .unwrap()
}

use super::{binaries::*, constants::*};
use yolo_distribution_bundle::*;

#[test]
fn runtime_verifies_its_bytes_for_each_target_and_rejects_corruption() {
    for target in all_targets() {
        let payloads = ArtifactRole::ALL.map(|role| {
            let mut bytes = binary(target, true);
            bytes.extend_from_slice(role.as_str().as_bytes());
            bytes
        });
        let [runtime] = payloads.each_ref().map(|bytes| {
            ArtifactDescriptor::new(yolo_crypto::sha256_hex(bytes), bytes.len() as u64).unwrap()
        });
        let bundle =
            BundleDescriptor::new(REVISION, VERSION, target, BundleArtifacts { runtime }).unwrap();
        let encoded = serde_json::to_vec(&bundle).unwrap();
        let decoded: BundleDescriptor = serde_json::from_slice(&encoded).unwrap();
        for (role, bytes) in ArtifactRole::ALL.into_iter().zip(&payloads) {
            assert_eq!(decoded.verify_artifact(role, bytes), Ok(()));
            let mut corrupt = bytes.clone();
            *corrupt.last_mut().unwrap() ^= SINGLE_BYTE;
            assert_eq!(
                decoded.verify_artifact(role, &corrupt),
                Err(BundleError::ArtifactIntegrity)
            );
            assert_eq!(
                decoded.verify_artifact(role, &[]),
                Err(BundleError::ArtifactIntegrity)
            );
        }
        for other_target in all_targets() {
            if other_target == target {
                continue;
            }
            let wrong = BundleDescriptor::new(
                REVISION,
                VERSION,
                other_target,
                bundle.fields().artifacts.clone(),
            )
            .unwrap();
            for (role, bytes) in ArtifactRole::ALL.into_iter().zip(&payloads) {
                assert_eq!(
                    wrong.verify_artifact(role, bytes),
                    Err(BundleError::ArtifactTarget)
                );
            }
        }
    }
}

#[test]
fn matching_digest_does_not_admit_non_executable_payload() {
    let bytes = PATH_ESCAPE.as_bytes();
    let artifact =
        ArtifactDescriptor::new(yolo_crypto::sha256_hex(bytes), bytes.len() as u64).unwrap();
    let bundle = BundleDescriptor::new(
        REVISION,
        VERSION,
        super::fixtures::target(),
        BundleArtifacts { runtime: artifact },
    )
    .unwrap();
    for role in ArtifactRole::ALL {
        assert_eq!(
            bundle.verify_artifact(role, bytes),
            Err(BundleError::ArtifactTarget)
        );
    }
}

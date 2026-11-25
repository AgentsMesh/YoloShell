use super::{constants::*, fixtures::*};
use yolo_distribution_bundle::*;

#[test]
fn artifact_descriptors_reject_malformed_digests_and_out_of_budget_sizes() {
    assert_eq!(
        MAX_ARTIFACT_BYTES,
        yolo_build_image::MAX_EXECUTABLE_IMAGE_BYTES
    );
    let image =
        yolo_build_image::BuildImage::new(RUNTIME_DIGEST.into(), MAX_ARTIFACT_BYTES).unwrap();
    assert_eq!(image.size_bytes(), MAX_ARTIFACT_BYTES);
    for digest in [
        String::new(),
        RUNTIME_DIGEST.to_uppercase(),
        INVALID_DIGEST.into(),
        RUNTIME_DIGEST[..RUNTIME_DIGEST.len() - 1].into(),
    ] {
        assert_eq!(
            ArtifactDescriptor::new(digest, RUNTIME_BYTES),
            Err(BundleError::Digest)
        );
    }
    for size in [0, MAX_ARTIFACT_BYTES + 1] {
        assert_eq!(
            ArtifactDescriptor::new(RUNTIME_DIGEST, size),
            Err(BundleError::Size)
        );
    }
    assert_eq!(
        ArtifactDescriptor::new(RUNTIME_DIGEST, MAX_ARTIFACT_BYTES)
            .unwrap()
            .size_bytes(),
        MAX_ARTIFACT_BYTES
    );
}

#[test]
fn constructor_and_wire_dtos_enforce_version_revision_and_schema() {
    for version in [
        String::new(),
        INVALID_VERSION.into(),
        "v".repeat(MAX_VERSION_BYTES + 1),
    ] {
        assert_eq!(
            BundleDescriptor::new(REVISION, version, target(), artifacts()),
            Err(BundleError::Version)
        );
    }
    for revision in [
        String::new(),
        REVISION.to_uppercase(),
        INVALID_DIGEST.into(),
        REVISION[..REVISION.len() - 1].into(),
    ] {
        assert_eq!(
            BundleDescriptor::new(revision, VERSION, target(), artifacts()),
            Err(BundleError::SourceRevision)
        );
    }
    assert!(BundleDescriptor::new(RUNTIME_DIGEST, "v1.2.3-rc.1", target(), artifacts()).is_ok());
    assert!(
        BundleDescriptor::new(
            REVISION,
            "v".repeat(MAX_VERSION_BYTES),
            target(),
            artifacts()
        )
        .is_ok()
    );
    let mut fields = descriptor().fields().clone();
    fields.schema_version += 1;
    assert_eq!(BundleDescriptor::try_from(fields), Err(BundleError::Schema));
    for build_id in [String::new(), RUNTIME_DIGEST.into(), PATH_ESCAPE.into()] {
        let mut fields = descriptor().fields().clone();
        fields.build_id = build_id;
        assert_eq!(
            BundleDescriptor::try_from(fields),
            Err(BundleError::BuildIdentity)
        );
    }
}

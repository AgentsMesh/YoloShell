use super::{constants::*, fixtures::*};
use std::path::Path;
use yolo_distribution_bundle::*;

#[test]
fn canonical_identity_matches_independently_encoded_golden_and_pins_all_paths() {
    let bundle = descriptor();
    assert_eq!(bundle.build_id(), GOLDEN_BUILD_ID);
    assert_eq!(bundle.source_revision(), REVISION);
    assert_eq!(bundle.version(), VERSION);
    assert_eq!(bundle.target(), target());
    assert_eq!(bundle.fields().schema_version, BUNDLE_SCHEMA_VERSION);
    assert_eq!(
        bundle.release_relative_path(),
        Path::new(RELEASES_DIRECTORY).join(GOLDEN_BUILD_ID)
    );
    assert_eq!(
        bundle.release_directory(Path::new(INSTALL_ROOT)),
        Path::new(INSTALL_ROOT).join(bundle.release_relative_path())
    );
    for role in ArtifactRole::ALL {
        assert_eq!(
            Path::new(role.relative_path()),
            Path::new(BIN_DIRECTORY).join(role.executable_name())
        );
        assert_eq!(
            bundle.executable_path(Path::new(INSTALL_ROOT), role),
            bundle
                .release_directory(Path::new(INSTALL_ROOT))
                .join(role.relative_path())
        );
        assert_eq!(
            bundle.artifact(role),
            bundle.fields().artifacts.artifact(role)
        );
        assert_eq!(serde_json::to_value(role).unwrap(), role.as_str());
    }
    let fields: BundleFields = bundle.clone().into();
    assert_eq!(BundleDescriptor::try_from(fields).unwrap(), bundle);
    let artifact: ArtifactFields = bundle.artifact(ArtifactRole::Runtime).clone().into();
    assert_eq!(artifact.sha256, RUNTIME_DIGEST);
    assert_eq!(artifact.size_bytes, RUNTIME_BYTES);
    assert_eq!(bundle.artifact(ArtifactRole::Runtime).fields(), &artifact);
}

#[test]
fn each_declared_identity_input_changes_the_build_and_tampering_is_rejected() {
    let original = descriptor();
    let mut variants = Vec::new();
    let fields = original.fields();
    let mut value = fields.clone();
    value.source_revision = OTHER_REVISION.into();
    variants.push(value);
    let mut value = fields.clone();
    value.version = OTHER_VERSION.into();
    variants.push(value);
    let mut value = fields.clone();
    value.target.platform = Platform::MacOs;
    variants.push(value);
    let mut value = fields.clone();
    value.target.architecture = Architecture::Aarch64;
    variants.push(value);
    for role in ArtifactRole::ALL {
        for replace_digest in [false, true] {
            let mut value = fields.clone();
            let artifact = value.artifacts.artifact(role);
            let replacement = ArtifactDescriptor::new(
                if replace_digest {
                    GOLDEN_BUILD_ID
                } else {
                    artifact.sha256()
                },
                artifact.size_bytes() + u64::from(!replace_digest),
            )
            .unwrap();
            match role {
                ArtifactRole::Runtime => value.artifacts.runtime = replacement,
            }
            variants.push(value);
        }
    }
    for changed in variants {
        assert_eq!(
            BundleDescriptor::try_from(changed.clone()),
            Err(BundleError::BuildIdentity)
        );
        let fresh = BundleDescriptor::new(
            changed.source_revision,
            changed.version,
            changed.target,
            changed.artifacts,
        )
        .unwrap();
        assert_ne!(fresh.build_id(), original.build_id());
    }
}

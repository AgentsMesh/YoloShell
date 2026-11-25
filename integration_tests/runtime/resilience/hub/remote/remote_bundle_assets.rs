use super::constants::*;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
use uuid::Uuid;
use yolo_distribution_bundle::{
    ArtifactDescriptor, ArtifactRole, BIN_DIRECTORY, BundleArtifacts, BundleDescriptor,
    MANIFEST_FILE_NAME,
};
use yolo_distribution_publication_protocol::{PublicationRequest, STAGE_PREFIX, Selection};

pub fn stage(
    root: &Path,
    expected: Option<Selection>,
    replace_runtime: bool,
) -> PublicationRequest {
    stage_version(
        root,
        expected,
        VERSION,
        if replace_runtime {
            packaged_source
        } else {
            source
        },
    )
}

fn stage_version(
    root: &Path,
    expected: Option<Selection>,
    version: &str,
    sources: fn(ArtifactRole) -> PathBuf,
) -> PublicationRequest {
    directory(root);
    let stage = Uuid::new_v4();
    let path = root.join(format!("{STAGE_PREFIX}{stage}"));
    directory(&path);
    directory(&path.join(BIN_DIRECTORY));
    let mut artifacts = Vec::new();
    let mut target = None;
    for role in ArtifactRole::ALL {
        let source = sources(role);
        let output = path.join(role.relative_path());
        fs::copy(source, &output).unwrap();
        fs::set_permissions(&output, fs::Permissions::from_mode(EXECUTABLE_MODE)).unwrap();
        let bytes = fs::read(&output).unwrap();
        let actual_target = yolo_distribution_bundle::binary_target(&bytes).unwrap();
        assert!(target.is_none_or(|expected| expected == actual_target));
        target = Some(actual_target);
        artifacts.push(
            ArtifactDescriptor::new(yolo_crypto::sha256_hex(&bytes), bytes.len() as u64).unwrap(),
        );
    }
    let [runtime]: [ArtifactDescriptor; ArtifactRole::ALL.len()] = artifacts.try_into().unwrap();
    let descriptor = BundleDescriptor::new(
        SOURCE_REVISION,
        version,
        target.unwrap(),
        BundleArtifacts { runtime },
    )
    .unwrap();
    let manifest = path.join(MANIFEST_FILE_NAME);
    fs::write(&manifest, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    fs::set_permissions(&manifest, fs::Permissions::from_mode(MANIFEST_MODE)).unwrap();
    PublicationRequest::new(root, Uuid::new_v4(), stage, descriptor, expected).unwrap()
}

/// Builds a local upload source; this does not publish or execute staged artifacts.
pub async fn upload_source(root: &Path) -> yolo_distribution_bundle::BundleSource {
    directory(root);
    let mut version = None;
    for role in ArtifactRole::ALL {
        let executable = packaged_source(role);
        let mut command = std::process::Command::new(&executable);
        command.arg(RUNTIME_VERSION_ARGUMENT);
        let result = super::process::invoke(&mut command, root, None).await;
        result.success();
        let text = std::str::from_utf8(&result.output).unwrap();
        let prefix = format!("{} ", role.executable_name());
        let actual = text
            .strip_prefix(&prefix)
            .and_then(|text| text.strip_suffix('\n'))
            .unwrap_or_else(|| panic!("{role:?}: {text:?}"));
        assert!(!actual.is_empty() && !actual.chars().any(char::is_whitespace));
        if let Some(expected) = &version {
            assert_eq!(expected, actual);
        }
        version = Some(actual.to_owned());
    }
    let request = stage_version(root, None, &version.unwrap(), packaged_source);
    yolo_distribution_bundle::BundleSource::new(
        root.join(format!("{STAGE_PREFIX}{}", request.stage())),
        request.descriptor().clone(),
    )
    .unwrap()
}

fn source(role: ArtifactRole) -> PathBuf {
    match role {
        ArtifactRole::Runtime => crate::runtime_binary(),
    }
}

fn packaged_source(role: ArtifactRole) -> PathBuf {
    match role {
        ArtifactRole::Runtime => {
            crate::bazel_runfile(&std::env::var(BUNDLE_HOST_BINARY_ENV).unwrap()).unwrap()
        }
    }
}

pub fn directory(path: &Path) {
    fs::create_dir_all(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(DIRECTORY_MODE)).unwrap();
}

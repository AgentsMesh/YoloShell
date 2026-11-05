use super::TrustedBazel;
use anyhow::Result;
use std::path::{Path, PathBuf};
use yolo_release_contracts::{BAZEL_VERSION_ARGUMENT, PINNED_BAZEL_SHA256, PINNED_BAZEL_VERSION};
use yolo_release_evidence_model::BazelLauncherEvidence;

impl TrustedBazel {
    pub fn fixture(path: PathBuf) -> Self {
        Self {
            _root: None,
            root_handle: None,
            root_seal: None,
            pinned_file: None,
            pinned_identity: None,
            evidence: BazelLauncherEvidence {
                canonical_path: path.display().to_string(),
                execution_path: path.display().to_string(),
                sha256: PINNED_BAZEL_SHA256.to_owned(),
                version: PINNED_BAZEL_VERSION.to_owned(),
            },
            path,
        }
    }

    pub fn validate_identity_for_test(
        path: &Path,
        expected_digest: &str,
        expected_version: &str,
    ) -> Result<Self> {
        Self::validate_identity(
            path,
            expected_digest,
            BAZEL_VERSION_ARGUMENT,
            expected_version,
        )
    }

    pub fn validate_identity_with_argument_for_test(
        path: &Path,
        expected_digest: &str,
        version_argument: &str,
        expected_version: &str,
    ) -> Result<Self> {
        Self::validate_identity(path, expected_digest, version_argument, expected_version)
    }
}

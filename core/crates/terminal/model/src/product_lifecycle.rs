use super::{TerminalManifestDigest, TerminalRuntimeStatus};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalLifecycleSelection {
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub terminal_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_updated_at_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_status: Option<TerminalRuntimeStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_manifest_sha256: Option<TerminalManifestDigest>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "yolo_crypto::serde_uuid::option"
    )]
    pub expected_runtime_epoch: Option<Uuid>,
    #[serde(default)]
    pub require_runtime_absent: bool,
}

impl TerminalLifecycleSelection {
    pub fn selected(terminal_id: Uuid, updated_at_ms: i64, status: TerminalRuntimeStatus) -> Self {
        Self {
            terminal_id,
            expected_updated_at_ms: Some(updated_at_ms),
            expected_status: Some(status),
            expected_manifest_sha256: None,
            expected_runtime_epoch: None,
            require_runtime_absent: false,
        }
    }

    pub fn unconditional(terminal_id: Uuid) -> Self {
        Self {
            terminal_id,
            expected_updated_at_ms: None,
            expected_status: None,
            expected_manifest_sha256: None,
            expected_runtime_epoch: None,
            require_runtime_absent: false,
        }
    }

    pub fn at_runtime_epoch(terminal_id: Uuid, expected_runtime_epoch: Uuid) -> Self {
        Self::unconditional(terminal_id).with_expected_runtime_epoch(Some(expected_runtime_epoch))
    }

    pub fn with_expected_runtime_epoch(mut self, expected: Option<Uuid>) -> Self {
        self.expected_runtime_epoch = expected;
        self
    }

    pub fn with_expected_manifest_sha256(
        mut self,
        expected: Option<TerminalManifestDigest>,
    ) -> Self {
        self.expected_manifest_sha256 = expected;
        self
    }

    pub fn is_conditional(&self) -> bool {
        self.require_runtime_absent
            || self.expected_updated_at_ms.is_some()
            || self.expected_status.is_some()
            || self.expected_manifest_sha256.is_some()
    }

    pub fn is_exact_catalog_selection(&self) -> bool {
        self.expected_updated_at_ms.is_some()
            && self.expected_status.is_some()
            && self.expected_manifest_sha256.is_some()
            && self
                .expected_runtime_epoch
                .is_some_and(|epoch| !epoch.is_nil())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalDiscardRequest {
    #[serde(flatten)]
    pub selection: TerminalLifecycleSelection,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalTerminateRequest {
    #[serde(flatten)]
    pub selection: TerminalLifecycleSelection,
    #[serde(default)]
    pub force: bool,
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../unit_tests/product_lifecycle_inline_tests.rs"]
mod tests;

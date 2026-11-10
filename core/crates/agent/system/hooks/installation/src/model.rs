use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use yolo_hook_installation_contract::HookInstallSpec;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationStatus {
    Missing,
    Configured,
    Restricted,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preview {
    pub provider: String,
    pub path: PathBuf,
    pub revision: String,
    pub status: IntegrationStatus,
    pub requires_provider_trust: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Installation {
    pub preview: Preview,
    pub changed: bool,
    pub backup: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookProfile {
    pub provider: String,
    pub display_name: String,
    /// Directory below the provider's home directory that holds the hook
    /// file; empty for providers that keep it in the home directory itself.
    pub subdirectory: String,
    pub file_name: String,
    pub requires_provider_trust: bool,
}

impl From<&HookInstallSpec> for HookProfile {
    fn from(spec: &HookInstallSpec) -> Self {
        Self {
            provider: spec.provider.into(),
            display_name: spec.display_name.into(),
            subdirectory: spec.subdirectory.into(),
            file_name: spec.file_name.into(),
            requires_provider_trust: spec.requires_provider_trust,
        }
    }
}

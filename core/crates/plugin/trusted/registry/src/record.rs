use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use yolo_plugin_api::Platform;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartupStatus {
    pub success: bool,
    pub finished_at_unix_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryRecord {
    pub id: String,
    pub source: PathBuf,
    pub digest: String,
    pub enabled: bool,
    pub platforms: Vec<Platform>,
    #[serde(default)]
    pub installed_at_unix_ms: u64,
    #[serde(default)]
    pub last_startup: Option<StartupStatus>,
}

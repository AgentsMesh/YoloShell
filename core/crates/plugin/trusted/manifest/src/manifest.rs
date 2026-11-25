use serde::{Deserialize, Serialize};

use crate::{Action, ClientApiContract, Command, Platform, WebMetadata};

pub const MANIFEST_FILE: &str = "yolo-plugin.json";
pub const SCHEMA_VERSION: u16 = 1;
pub const MAX_MANIFEST_BYTES: u64 = 256 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedPluginManifest {
    pub schema_version: u16,
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    pub platforms: Vec<Platform>,
    #[serde(default)]
    pub startup: Option<Command>,
    #[serde(default)]
    pub actions: Vec<Action>,
    #[serde(default)]
    pub web: Option<WebMetadata>,
    pub api: ClientApiContract,
}

use serde::{Deserialize, Serialize};
use yolo_plugin_api::Platform;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Executable {
    YoloImage,
    BundleRelative { path: String },
    Absolute { path: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessEntrypoint {
    pub platform: Platform,
    pub executable: Executable,
    pub arguments: Vec<String>,
}

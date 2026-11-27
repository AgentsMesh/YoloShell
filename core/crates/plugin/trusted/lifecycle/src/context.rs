use std::path::PathBuf;

use yolo_hub_identity::HubId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LifecycleContext {
    pub plugin_id: String,
    pub root: PathBuf,
    pub data: PathBuf,
    pub log: PathBuf,
    pub hub_socket: PathBuf,
    pub hub_id: Option<HubId>,
    pub action_id: Option<String>,
}

use crate::constants::{STATE_CONTRACT_FILE, STATE_NAMESPACE, STATE_SCHEMA_VERSION};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::Path;
use yolo_hub_rpc_protocol::HUB_PROTOCOL_VERSION as PUBLIC_HUB_PROTOCOL_VERSION;
use yolo_paths::namespace_lease::NamespaceLease;

const CONTRACT_SCHEMA: u16 = 1;
const INCOMPATIBLE: &str = "Hub state belongs to an incompatible Yolo image; stop the old Hub and migrate or quarantine this state";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Contract {
    schema: u16,
    hub_protocol: u16,
    state_schema: u16,
}

pub fn open_or_validate(namespace: &NamespaceLease, path: &Path, fresh: bool) -> Result<()> {
    let file = path.join(STATE_CONTRACT_FILE);
    if fresh {
        return namespace.write_json(
            &file,
            &Contract {
                schema: CONTRACT_SCHEMA,
                hub_protocol: PUBLIC_HUB_PROTOCOL_VERSION,
                state_schema: STATE_SCHEMA_VERSION,
            },
        );
    }
    let contract: Contract = namespace
        .read_json(&file)
        .map_err(|_| anyhow::anyhow!(INCOMPATIBLE))?;
    ensure!(contract.schema == CONTRACT_SCHEMA, INCOMPATIBLE);
    ensure!(
        contract.hub_protocol == PUBLIC_HUB_PROTOCOL_VERSION,
        INCOMPATIBLE
    );
    ensure!(contract.state_schema == STATE_SCHEMA_VERSION, INCOMPATIBLE);
    Ok(())
}

/// Validate a retained state contract without taking the Hub singleton lock.
/// Upgrade admission uses this read-only probe before it considers replacing a
/// service. It intentionally never creates, rewrites, or migrates state.
pub fn validate_path(path: &Path) -> Result<()> {
    let directory = yolo_paths::directory::PrivateDirectory::open(path)?;
    let namespace = NamespaceLease::from_directories(vec![(STATE_NAMESPACE, directory)])?;
    open_or_validate(&namespace, path, false)
}

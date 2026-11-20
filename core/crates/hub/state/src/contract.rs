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

pub fn open_or_migrate(namespace: &NamespaceLease, path: &Path, fresh: bool) -> Result<()> {
    if fresh {
        return write_contract(namespace, path);
    }
    let contract = read(namespace, path)?;
    validate(&contract)?;
    if contract.hub_protocol < PUBLIC_HUB_PROTOCOL_VERSION {
        write_contract(namespace, path)?;
    }
    Ok(())
}

/// Validate a retained state contract without taking the Hub singleton lock.
/// Upgrade admission uses this read-only probe before it considers replacing a
/// service. It accepts older Hub protocol versions but never rewrites state.
pub fn validate_path(path: &Path) -> Result<()> {
    let directory = yolo_paths::directory::PrivateDirectory::open(path)?;
    let namespace = NamespaceLease::from_directories(vec![(STATE_NAMESPACE, directory)])?;
    validate(&read(&namespace, path)?)
}

fn read(namespace: &NamespaceLease, path: &Path) -> Result<Contract> {
    namespace
        .read_json(&path.join(STATE_CONTRACT_FILE))
        .map_err(|_| anyhow::anyhow!(INCOMPATIBLE))
}

fn write_contract(namespace: &NamespaceLease, path: &Path) -> Result<()> {
    namespace.write_json(
        &path.join(STATE_CONTRACT_FILE),
        &Contract {
            schema: CONTRACT_SCHEMA,
            hub_protocol: PUBLIC_HUB_PROTOCOL_VERSION,
            state_schema: STATE_SCHEMA_VERSION,
        },
    )
}

fn validate(contract: &Contract) -> Result<()> {
    ensure!(contract.schema == CONTRACT_SCHEMA, INCOMPATIBLE);
    ensure!(contract.state_schema == STATE_SCHEMA_VERSION, INCOMPATIBLE);
    // An older protocol is an upgrade; a newer one means this image is older
    // than the state and must not interpret it.
    ensure!(
        contract.hub_protocol <= PUBLIC_HUB_PROTOCOL_VERSION,
        INCOMPATIBLE
    );
    Ok(())
}

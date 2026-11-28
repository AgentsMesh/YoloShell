use crate::{RemoteDirectories, constants::*, not_in_releases};
use anyhow::{Result, ensure};
use yolo_distribution_bundle::ArtifactRole;
use yolo_distribution_release_store::ReleaseStore;
use yolo_hub_local_startup::Launch;
use yolo_remote_provisioning_protocol::RemoteBootstrapRequest;
use yolo_hub_state::HubState;

pub(crate) fn prepare(
    directories: &RemoteDirectories,
    request: &RemoteBootstrapRequest,
) -> Result<Launch> {
    let store = ReleaseStore::open(&directories.install_root)?;
    not_in_releases(&directories.state, store.root())?;
    not_in_releases(&directories.host_home, store.root())?;
    let release = store
        .inspect_release(&request.fields().bundle)?
        .ok_or_else(|| anyhow::anyhow!(BUNDLE_MISSING))?;
    if let Some(expected) = request.fields().expected_peer {
        ensure!(
            HubState::inspect_peer_identity(&directories.state, &directories.host_home)?
                == Some(expected),
            PIN_MISMATCH
        );
    }
    Ok(Launch {
        state: directories.state.clone(),
        host_home: directories.host_home.clone(),
        host_executable: release.join(ArtifactRole::Runtime.relative_path()),
        hub_executable: release.join(ArtifactRole::Runtime.relative_path()),
    })
}

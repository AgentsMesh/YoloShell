use crate::{RemoteDirectories, constants::*, require_budget};
use anyhow::{Result, ensure};
use tokio::time::{Instant, sleep_until};
use yolo_hub_enrollment_store::{StartupAdmission, admit_startup};
use yolo_remote_provisioning_protocol::RemoteBootstrapRequest;
use yolo_hub_state::HubState;
use yolo_paths::directory::PrivateDirectory;

pub(crate) async fn acquire(
    directories: &RemoteDirectories,
    request: &RemoteBootstrapRequest,
    deadline: Instant,
) -> Result<StartupAdmission> {
    require_budget(deadline)?;
    let path = yolo_paths::platform::platform_path(&directories.state);
    let parent = if let Some(peer) = request.fields().expected_peer {
        let parent = PrivateDirectory::open(&path)?;
        ensure!(
            HubState::inspect_peer_identity(&path, &directories.host_home)? == Some(peer),
            PIN_MISMATCH
        );
        parent
    } else {
        PrivateDirectory::open_or_create_strict(&path)?
    };
    loop {
        require_budget(deadline)?;
        parent.verify_path()?;
        if let Some(admission) = admit_startup(parent.try_clone()?, request)? {
            admission.verify()?;
            return Ok(admission);
        }
        sleep_until((Instant::now() + RECONCILE_POLL).min(deadline)).await;
    }
}

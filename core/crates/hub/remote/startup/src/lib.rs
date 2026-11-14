mod admission;
mod configuration;
mod constants;
mod control;
mod enrollment;
mod launch;
mod receipt;
mod reconcile;
mod recovery;

use anyhow::{Result, ensure};
pub use recovery::{inspect, recover};
use std::path::{Path, PathBuf};
use tokio::time::Instant;
use yolo_remote_provisioning_protocol::{RemoteBootstrapReceipt, RemoteBootstrapRequest};

#[derive(Clone, Debug)]
pub struct RemoteDirectories {
    pub install_root: PathBuf,
    pub state: PathBuf,
    pub host_home: PathBuf,
}

/// Startup commits and configuration changes are never rolled back by SSH loss.
pub async fn ensure_running(
    directories: &RemoteDirectories,
    request: &RemoteBootstrapRequest,
    deadline: Instant,
) -> Result<RemoteBootstrapReceipt> {
    require_budget(deadline)?;
    if inspect(directories, request)?.is_some() {
        return enrollment::ready(recover(directories, request, deadline).await?);
    }
    let admission = admission::acquire(directories, request, deadline).await?;
    if inspect(directories, request)?.is_some() {
        return enrollment::ready(recover(directories, request, deadline).await?);
    }
    admission.verify()?;
    let launch = launch::prepare(directories, request)?;
    admission.verify()?;
    let connection = yolo_hub_local_startup::ensure_running_with_peer_in(
        launch,
        request.fields().expected_peer,
        admission.parent_directory()?,
        deadline,
    )
    .await?;
    admission.verify()?;
    enrollment::admitted(directories, &connection, request, admission, deadline).await
}

pub(crate) fn require_budget(deadline: Instant) -> Result<()> {
    ensure!(Instant::now() < deadline, constants::DEADLINE);
    Ok(())
}

pub(crate) fn not_in_releases(path: &Path, root: &Path) -> Result<()> {
    yolo_paths::validation::validate_absolute(path)?;
    let path = yolo_paths::coordination::canonical_logical_path(path)?;
    let releases = yolo_paths::coordination::canonical_logical_path(
        &root.join(yolo_distribution_bundle::RELEASES_DIRECTORY),
    )?;
    ensure!(!path.starts_with(releases), constants::MUTABLE_RELEASE_PATH);
    Ok(())
}

#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/lib.rs"]
mod tests;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/lib.rs"]
mod integration_tests;

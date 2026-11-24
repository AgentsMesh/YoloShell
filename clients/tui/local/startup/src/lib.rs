//! CLI composition for the existing owned local-Hub bootstrap helper.
mod bootstrap;
mod constants;
mod helpers;
mod hostname;
mod receipt;
mod signals;

use anyhow::Result;
pub use helpers::Helpers;
pub use hostname::local_hostname;
use std::path::Path;
use yolo_hub_local_startup::ConnectionDocument;
use yolo_local_profile::LocalProfile;

pub struct AutomaticConnection {
    pub connection: ConnectionDocument,
    cancellation: signals::Cancellation,
}
impl AutomaticConnection {
    /// Keep this owner alive through the TUI so signals received between
    /// bootstrap and terminal setup cannot be silently discarded.
    pub async fn cancelled(&mut self) {
        self.cancellation.cancelled().await;
    }
}

pub async fn automatic(profile_path: Option<&Path>) -> Result<AutomaticConnection> {
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = profile_path;
        anyhow::bail!(constants::AUTOMATIC_UNSUPPORTED)
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let profile = match profile_path {
            Some(path) => LocalProfile::load(path)?,
            None => yolo_local_profile::current()?,
        };
        let executable = std::env::current_exe()?;
        let mut cancellation = signals::Cancellation::install()?;
        let result = tokio::select! {
            result = prepare(&profile, &executable) => result,
            _ = cancellation.cancelled() => Err(anyhow::anyhow!(constants::CANCELLED)),
        };
        yolo_process::wait_for_detached_termination_delivery(
            std::time::Instant::now() + yolo_hub_local_startup::BOOTSTRAP_RETIRE_TIMEOUT,
        )?;
        Ok(AutomaticConnection {
            connection: result?,
            cancellation,
        })
    }
}

/// Caller owns cancellation by retaining or dropping this future. Only its
/// short-lived bootstrap child is scoped; committed services keep their custody.
pub async fn prepare(profile: &LocalProfile, executable: &Path) -> Result<ConnectionDocument> {
    profile.validate()?;
    let helpers = Helpers::discover(executable)?;
    let deadline = tokio::time::Instant::now() + yolo_hub_local_startup::STARTUP_TIMEOUT;
    bootstrap::run(profile, &helpers, deadline).await
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/lib.rs"]
mod integration_tests;
#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

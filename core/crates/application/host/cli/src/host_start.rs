use anyhow::Result;
use std::time::Duration;
use tokio::time::Instant;
use yolo_runtime::host::{Host, HostConfig, HostInstanceBusy};

const INSTANCE_LOCK_WAIT: Duration = Duration::from_secs(5);
const INSTANCE_LOCK_POLL: Duration = Duration::from_millis(10);

pub(super) async fn serve(
    home: std::path::PathBuf,
    socket: Option<std::path::PathBuf>,
    guarded: bool,
) -> Result<()> {
    let receiver = guarded
        .then(yolo_startup_guard::Receiver::from_stdin)
        .transpose()?;
    super::hook_installation::migrate_installed();
    let stop = async move {
        match receiver {
            Some(receiver) => receiver.cancellation().await,
            None => std::future::pending().await,
        }
    };
    tokio::pin!(stop);
    let mut config = HostConfig::with_provider_runtime(
        home,
        std::sync::Arc::new(yolo_runtime::agent::builtin_provider_runtime()),
    );
    config.runtime_executable = Some(std::env::current_exe()?);
    config.socket = socket;
    let host = tokio::select! {
        result = open(config) => result?,
        _ = &mut stop => anyhow::bail!(yolo_startup_guard::CANCELLED),
    };
    host.settle_launching_creations()?;
    host.serve_with_stop(stop).await
}

pub(super) async fn open(config: HostConfig) -> Result<Host> {
    open_until(config, Instant::now() + INSTANCE_LOCK_WAIT).await
}

async fn open_until(config: HostConfig, deadline: Instant) -> Result<Host> {
    loop {
        match Host::open(config.clone()) {
            Ok(host) => return Ok(host),
            Err(error) if error.is::<HostInstanceBusy>() && Instant::now() < deadline => {
                tokio::time::sleep_until(deadline.min(Instant::now() + INSTANCE_LOCK_POLL)).await;
            }
            Err(error) => return Err(error),
        }
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/host_start.rs"]
mod tests;

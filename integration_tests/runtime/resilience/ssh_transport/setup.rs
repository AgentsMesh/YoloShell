use super::constants::TRANSPORT_EVENT_TIMEOUT;
use super::factory::LiveSshFactory;
use super::runtime_guard::DetachedRuntimeGuard;
use super::sshd::SshdFixture;
use anyhow::{Result, ensure};
use std::sync::Arc;
use yolo_remote::{
    ChannelIdentity, ChannelPurpose, MultiplexedSshConnection,
    RemoteCleanupRegistry, RemoteRuntimeSpec, RuntimeChannelRequest,
};

#[path = "setup_guard.rs"]
mod guard;
#[path = "setup_inventory.rs"]
mod inventory;
#[path = "setup_provisional.rs"]
mod provisional;
pub(super) use inventory::bind_runtime_inventory;
use provisional::ProvisionalRuntimeGuard;

const INITIAL_CONNECT_TIMEOUT_DETAIL: &str = "initial SSH connection timed out";
const INITIAL_CONNECT_FAILURE_DETAIL: &str = "initial SSH connection failed";
const INITIAL_RUNTIME_FAILURE_DETAIL: &str = "initial SSH runtime open failed";
const DETACHED_GUARD_FAILURE_DETAIL: &str = "establish detached runtime cleanup custody";
const OPENED_CONNECTION_MISMATCH_DETAIL: &str =
    "opened runtime channel belongs to another SSH connection";
const OPENED_PURPOSE_MISMATCH_DETAIL: &str = "opened runtime channel has the wrong purpose";
const CLEANUP_FAILURES_DETAIL: &str = "cleanup failures";
const CLEANUP_FAILURE_SEPARATOR: &str = "; ";
const PROVISIONAL_GUARD_START_FAILURE_DETAIL: &str =
    "start provisional detached runtime cleanup guard";

pub(super) struct OpenedRuntime { pub stream: yolo_runtime_rpc::RuntimeConnection }

pub(super) async fn open_initial(
    sshd: &SshdFixture,
    factory: &LiveSshFactory,
    runtime: &RemoteRuntimeSpec,
) -> (
    Arc<dyn MultiplexedSshConnection>,
    OpenedRuntime,
    DetachedRuntimeGuard,
) {
    open_initial_inner(sshd, factory, runtime, None).await
}

async fn open_initial_inner(
    sshd: &SshdFixture,
    factory: &LiveSshFactory,
    runtime: &RemoteRuntimeSpec,
    probe_barrier: Option<provisional::ProbeBarrier>,
) -> (
    Arc<dyn MultiplexedSshConnection>,
    OpenedRuntime,
    DetachedRuntimeGuard,
) {
    let connection = tokio::time::timeout(
        TRANSPORT_EVENT_TIMEOUT,
        factory.connect_once(RemoteCleanupRegistry::default()),
    )
    .await
    .unwrap_or_else(|error| {
        panic!(
            "{INITIAL_CONNECT_TIMEOUT_DETAIL}: {error}; {}",
            sshd.diagnostics()
        )
    })
    .unwrap_or_else(|error| {
        panic!(
            "{INITIAL_CONNECT_FAILURE_DETAIL}: {error}; {}",
            sshd.diagnostics()
        )
    });
    let provisional = provisional_guard(sshd, probe_barrier);
    let result = tokio::time::timeout(
        TRANSPORT_EVENT_TIMEOUT,
        connection.open_runtime(&RuntimeChannelRequest::control(runtime.clone())),
    )
    .await;
    let opened = match result {
        Ok(Ok(opened)) => opened,
        Ok(Err(error)) => fail_runtime(sshd, error.to_string(), provisional.acquire().await).await,
        Err(error) => fail_runtime(sshd, error.to_string(), provisional.acquire().await).await,
    };
    let mut guard = match provisional.acquire().await {
        Ok(guard) => guard,
        Err(failure) => {
            drop(opened);
            fail_runtime(sshd, DETACHED_GUARD_FAILURE_DETAIL.into(), Err(failure)).await
        }
    };
    if let Err(error) = validate_opened_channel(connection.info().connection_id, opened.identity) {
        drop(opened);
        fail_runtime(sshd, format!("{error:#}"), Ok(guard)).await;
    }
    let connection_rpc = match yolo_remote::probe_runtime_channel(opened.stream, runtime,
        yolo_remote::RemoteDeadline::operation()).await {
        Ok(connection) => connection,
        Err(error) => fail_runtime(sshd, error.to_string(), Ok(guard)).await,
    };
    let mut opened = OpenedRuntime { stream: connection_rpc };
    if let Err(error) = bind_runtime_inventory(&mut opened.stream, &mut guard).await {
        drop(opened);
        fail_runtime(sshd, format!("{error:#}"), Ok(guard)).await;
    }
    (connection, opened, guard)
}

fn validate_opened_channel(
    expected_connection_id: uuid::Uuid,
    opened: ChannelIdentity,
) -> Result<()> {
    ensure!(
        opened.connection_id == expected_connection_id,
        OPENED_CONNECTION_MISMATCH_DETAIL
    );
    ensure!(
        opened.purpose == ChannelPurpose::RuntimeControl,
        OPENED_PURPOSE_MISMATCH_DETAIL
    );
    Ok(())
}

async fn fail_runtime(
    sshd: &SshdFixture,
    primary: String,
    guard: std::result::Result<DetachedRuntimeGuard, guard::GuardArmFailure>,
) -> ! {
    let mut cleanup = Vec::new();
    let mut guard = match guard {
        Ok(guard) => Some(guard),
        Err(failure) => {
            let (error, guard) = failure.into_parts();
            cleanup.push(format!("{error:#}"));
            guard
        }
    };
    if let Some(guard) = guard.as_mut()
        && let Err(error) = guard.shutdown().await
    {
        cleanup.push(format!("{error:#}"));
    }
    let cleanup = cleanup.is_empty().then(String::new).unwrap_or_else(|| {
        format!(
            "; {CLEANUP_FAILURES_DETAIL}: {}",
            cleanup.join(CLEANUP_FAILURE_SEPARATOR)
        )
    });
    panic!(
        "{INITIAL_RUNTIME_FAILURE_DETAIL}: {primary}{cleanup}; sshd log: {}",
        sshd.diagnostics()
    )
}

fn provisional_guard(
    sshd: &SshdFixture,
    probe_barrier: Option<provisional::ProbeBarrier>,
) -> ProvisionalRuntimeGuard {
    match probe_barrier {
        Some(barrier) => ProvisionalRuntimeGuard::start_with_barrier(sshd.remote_home(), barrier),
        None => ProvisionalRuntimeGuard::start(sshd.remote_home()),
    }
    .expect(PROVISIONAL_GUARD_START_FAILURE_DETAIL)
}

#[cfg(test)]
async fn open_initial_with_probe_barrier(
    sshd: &SshdFixture,
    factory: &LiveSshFactory,
    runtime: &RemoteRuntimeSpec,
    barrier: provisional::ProbeBarrier,
) -> (
    Arc<dyn MultiplexedSshConnection>,
    OpenedRuntime,
    DetachedRuntimeGuard,
) {
    open_initial_inner(sshd, factory, runtime, Some(barrier)).await
}

#[cfg(all(test, target_os = "macos"))]
#[path = "setup_cancellation_tests.rs"]
mod cancellation_tests;
#[cfg(test)]
#[path = "setup_tests.rs"]
mod tests;

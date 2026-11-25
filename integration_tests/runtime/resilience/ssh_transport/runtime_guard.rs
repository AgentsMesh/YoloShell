use crate::support::RuntimeCleanup;
use anyhow::{Context, Result, ensure};
use std::path::Path;
use std::time::{Duration, Instant};
use yolo_process_test_support::{ProcessIdentity, ProcessIdentityObservation};
use yolo_runtime::model::HostStatus;
use yolo_runtime::paths::Paths;

const DETACHED_HOST_EXIT_TIMEOUT: Duration = Duration::from_secs(3);
const DETACHED_HOST_EXIT_POLL_INTERVAL: Duration = Duration::from_millis(20);
const DETACHED_HOME_CHANGED_DETAIL: &str = "detached runtime home changed";
const DETACHED_PROTOCOL_CHANGED_DETAIL: &str = "detached runtime protocol version changed";
const DETACHED_PID_CHANGED_DETAIL: &str = "detached runtime Host PID changed";
const DETACHED_BOOT_CHANGED_DETAIL: &str = "detached runtime boot identity changed";
const DETACHED_GENERATION_CHANGED_DETAIL: &str = "detached runtime controller generation changed";
const DETACHED_IDENTITY_CHANGED_DETAIL: &str = "detached runtime Host identity changed";
const DETACHED_SIGNAL_CONTEXT: &str = "signal detached runtime Host identity";
const DETACHED_HOST_SURVIVED_DETAIL: &str = "detached runtime Host identity survived cleanup";
const GRACEFUL_CLEANUP_FAILURE_CONTEXT: &str = "graceful detached Host cleanup failed first";

#[path = "runtime_guard_failures.rs"]
mod failures;
use failures::{finish_failures, remember_failure, report_drop_failures};
#[path = "runtime_guard_cleanup.rs"]
mod cleanup;

pub(super) struct DetachedRuntimeGuard {
    paths: Paths,
    host: ProcessIdentity,
    boot_id: Option<String>,
    controller_generation: Option<u64>,
    cleanup: Option<RuntimeCleanup>,
    armed: bool,
}

impl DetachedRuntimeGuard {
    pub(super) fn from_peer(expected_home: &Path, host: ProcessIdentity) -> Self {
        let paths = Paths::new(expected_home);
        Self {
            paths,
            host,
            boot_id: None,
            controller_generation: None,
            cleanup: None,
            armed: true,
        }
    }

    pub(super) fn verify_status(&mut self, status: &HostStatus) -> Result<()> {
        ensure!(
            status.protocol_version == yolo_runtime::model::PROTOCOL_VERSION,
            DETACHED_PROTOCOL_CHANGED_DETAIL
        );
        ensure!(
            status.home == self.paths.home(),
            DETACHED_HOME_CHANGED_DETAIL
        );
        ensure!(
            status.host_pid == self.host.pid,
            DETACHED_PID_CHANGED_DETAIL
        );
        ensure!(
            self.host.observe() == ProcessIdentityObservation::Matching,
            DETACHED_IDENTITY_CHANGED_DETAIL
        );
        bind_boot(&mut self.boot_id, &status.boot_id)?;
        bind_stable(
            &mut self.controller_generation,
            &status.controller_generation,
            DETACHED_GENERATION_CHANGED_DETAIL,
        )?;
        if self.cleanup.is_none() {
            let (cleanup, capture) = RuntimeCleanup::capture_for_boot(&self.paths, &status.boot_id);
            self.cleanup = Some(cleanup);
            capture?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(in super::super) fn disarm_for_test(&mut self) {
        self.armed = false;
    }
}

fn bind_boot(slot: &mut Option<String>, value: &str) -> Result<()> {
    use yolo_process_test_support::{BootIdentityRelation, boot_identity_relation};
    ensure!(
        boot_identity_relation(value, value) == BootIdentityRelation::Same,
        DETACHED_BOOT_CHANGED_DETAIL
    );
    match slot {
        Some(bound) => ensure!(
            boot_identity_relation(bound, value) == BootIdentityRelation::Same,
            DETACHED_BOOT_CHANGED_DETAIL
        ),
        None => *slot = Some(value.to_owned()),
    }
    Ok(())
}

fn bind_stable<T: Eq + Clone>(slot: &mut Option<T>, value: &T, detail: &'static str) -> Result<()> {
    match slot {
        Some(bound) => ensure!(bound == value, detail),
        None => *slot = Some(value.clone()),
    }
    Ok(())
}

fn signal_if_matching(identity: ProcessIdentity, signal: i32) -> Result<()> {
    match identity.signal(signal) {
        Ok(_) => Ok(()),
        Err(_) if identity.observe() == ProcessIdentityObservation::Gone => Ok(()),
        Err(error) => Err(error).context(DETACHED_SIGNAL_CONTEXT),
    }
}

fn wait_for_host_gone(identity: ProcessIdentity) -> Result<()> {
    let deadline = Instant::now() + DETACHED_HOST_EXIT_TIMEOUT;
    while identity.observe() != ProcessIdentityObservation::Gone && Instant::now() < deadline {
        std::thread::sleep(DETACHED_HOST_EXIT_POLL_INTERVAL);
    }
    ensure!(
        identity.observe() == ProcessIdentityObservation::Gone,
        "{DETACHED_HOST_SURVIVED_DETAIL}: {identity:?}"
    );
    Ok(())
}

async fn wait_for_host_gone_async(identity: ProcessIdentity) -> Result<()> {
    let deadline = tokio::time::Instant::now() + DETACHED_HOST_EXIT_TIMEOUT;
    while identity.observe() != ProcessIdentityObservation::Gone
        && tokio::time::Instant::now() < deadline
    {
        tokio::time::sleep(DETACHED_HOST_EXIT_POLL_INTERVAL).await;
    }
    ensure!(
        identity.observe() == ProcessIdentityObservation::Gone,
        "{DETACHED_HOST_SURVIVED_DETAIL}: {identity:?}"
    );
    Ok(())
}

async fn stop_host_async(identity: ProcessIdentity) -> Result<()> {
    let graceful = match signal_if_matching(identity, libc::SIGINT) {
        Ok(()) => wait_for_host_gone_async(identity).await,
        Err(error) => Err(error),
    };
    if graceful.is_ok() {
        return Ok(());
    }
    let graceful = graceful.unwrap_err();
    signal_if_matching(identity, libc::SIGKILL)
        .with_context(|| format!("{GRACEFUL_CLEANUP_FAILURE_CONTEXT}: {graceful:#}"))?;
    wait_for_host_gone_async(identity)
        .await
        .with_context(|| format!("{GRACEFUL_CLEANUP_FAILURE_CONTEXT}: {graceful:#}"))
}

fn cleanup_requires_retry(observation: ProcessIdentityObservation, cleanup_failed: bool) -> bool {
    observation != ProcessIdentityObservation::Gone || cleanup_failed
}

#[cfg(test)]
#[path = "runtime_guard_inventory_tests.rs"]
mod inventory_tests;
#[cfg(test)]
#[path = "runtime_guard_tests.rs"]
mod tests;

use super::constants::WATCHDOG_SETTLEMENT_TIMEOUT;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[path = "fault_contract.rs"]
mod contract;
#[path = "fault_witness.rs"]
mod witness;
use contract::*;
pub(super) use witness::{acknowledge_historical_witness, capture_historical_witness};
const PANIC_DETAIL: &str = "injected isolated test guard panic";
const FORCE_SIGNAL_HOOK_DETAIL: &str = "injected force-signal hook timed out";
const REFRESH_HANDOFF_HOOK_DETAIL: &str = "injected refresh-handoff hook timed out";
const DISCOVERY_ARM_HOOK_DETAIL: &str = "injected discovery-arm hook timed out";
const FORCE_SIGNAL_HOOK_POLL_INTERVAL: Duration = Duration::from_millis(10);
const EMPTY_MARKER: &[u8] = b"";
static FORCE_SIGNAL_HOOK_USED: AtomicBool = AtomicBool::new(false);
static REFRESH_HANDOFF_HOOK_USED: AtomicBool = AtomicBool::new(false);
static DISCOVERY_ARM_HOOK_USED: AtomicBool = AtomicBool::new(false);

pub(super) struct BarrierHook {
    request: OsString,
    acknowledgement: OsString,
}

pub(super) fn check() {
    let Some(trigger) = std::env::var_os(FAILURE_TRIGGER_ENVIRONMENT) else {
        return;
    };
    if !std::path::Path::new(&trigger).exists() {
        return;
    }
    match std::env::var(FAILURE_MODE_ENVIRONMENT).as_deref() {
        Ok(PANIC_MODE) => panic!("{PANIC_DETAIL}"),
        Ok(EXIT_MODE) => unsafe { libc::_exit(libc::EXIT_FAILURE) },
        _ => {}
    }
}

pub(super) fn remove_environment(command: &mut Command) {
    command
        .env_remove(FAILURE_MODE_ENVIRONMENT)
        .env_remove(FAILURE_TRIGGER_ENVIRONMENT)
        .env_remove(FORCE_SIGNAL_REQUEST_ENVIRONMENT)
        .env_remove(FORCE_SIGNAL_ACKNOWLEDGEMENT_ENVIRONMENT)
        .env_remove(REFRESH_HANDOFF_REQUEST_ENVIRONMENT)
        .env_remove(REFRESH_HANDOFF_ACKNOWLEDGEMENT_ENVIRONMENT)
        .env_remove(DISCOVERY_ARM_REQUEST_ENVIRONMENT)
        .env_remove(DISCOVERY_ARM_ACKNOWLEDGEMENT_ENVIRONMENT)
        .env_remove(HISTORICAL_WITNESS_IDENTITY_ENVIRONMENT)
        .env_remove(HISTORICAL_WITNESS_ACKNOWLEDGEMENT_ENVIRONMENT)
        .env_remove(REFRESH_POSTCHECK_WITNESS_ENVIRONMENT)
        .env_remove(DISCOVERY_WRITER_DESCRIPTOR_ENVIRONMENT);
}

pub(super) fn capture_force_signal_hook() -> Option<BarrierHook> {
    capture_barrier(
        FORCE_SIGNAL_REQUEST_ENVIRONMENT,
        FORCE_SIGNAL_ACKNOWLEDGEMENT_ENVIRONMENT,
    )
}

pub(super) fn capture_refresh_handoff_hook() -> Option<BarrierHook> {
    capture_barrier(
        REFRESH_HANDOFF_REQUEST_ENVIRONMENT,
        REFRESH_HANDOFF_ACKNOWLEDGEMENT_ENVIRONMENT,
    )
}

pub(super) fn capture_discovery_arm_hook() -> Option<BarrierHook> {
    capture_barrier(
        DISCOVERY_ARM_REQUEST_ENVIRONMENT,
        DISCOVERY_ARM_ACKNOWLEDGEMENT_ENVIRONMENT,
    )
}

pub(super) fn capture_refresh_postcheck_witness() -> Option<OsString> {
    std::env::var_os(REFRESH_POSTCHECK_WITNESS_ENVIRONMENT)
}

pub(super) fn forward_force_signal_hook(command: &mut Command, hook: Option<BarrierHook>) {
    forward_barrier(
        command,
        hook,
        FORCE_SIGNAL_REQUEST_ENVIRONMENT,
        FORCE_SIGNAL_ACKNOWLEDGEMENT_ENVIRONMENT,
    );
}

pub(super) fn forward_refresh_handoff_hook(command: &mut Command, hook: Option<BarrierHook>) {
    forward_barrier(
        command,
        hook,
        REFRESH_HANDOFF_REQUEST_ENVIRONMENT,
        REFRESH_HANDOFF_ACKNOWLEDGEMENT_ENVIRONMENT,
    );
}

pub(super) fn forward_discovery_arm_hook(command: &mut Command, hook: Option<BarrierHook>) {
    forward_barrier(
        command,
        hook,
        DISCOVERY_ARM_REQUEST_ENVIRONMENT,
        DISCOVERY_ARM_ACKNOWLEDGEMENT_ENVIRONMENT,
    );
}

pub(super) fn forward_refresh_postcheck_witness(command: &mut Command, witness: Option<OsString>) {
    if let Some(witness) = witness {
        command.env(REFRESH_POSTCHECK_WITNESS_ENVIRONMENT, witness);
    }
}

#[cfg(target_os = "macos")]
pub(super) fn forward_discovery_writer_descriptor(command: &mut Command, descriptor: libc::c_int) {
    command.env(
        DISCOVERY_WRITER_DESCRIPTOR_ENVIRONMENT,
        descriptor.to_string(),
    );
}

pub(super) fn before_force_signal() -> anyhow::Result<()> {
    run_barrier(
        capture_force_signal_hook(),
        &FORCE_SIGNAL_HOOK_USED,
        FORCE_SIGNAL_HOOK_DETAIL,
    )
    .map(|_activated| ())
}

pub(super) fn after_new_live_refresh() -> anyhow::Result<bool> {
    run_barrier(
        capture_refresh_handoff_hook(),
        &REFRESH_HANDOFF_HOOK_USED,
        REFRESH_HANDOFF_HOOK_DETAIL,
    )
}

pub(super) fn after_empty_settled_refresh() -> anyhow::Result<bool> {
    run_barrier(
        capture_discovery_arm_hook(),
        &DISCOVERY_ARM_HOOK_USED,
        DISCOVERY_ARM_HOOK_DETAIL,
    )
}

pub(super) fn record_refresh_postcheck() -> anyhow::Result<()> {
    let Some(path) = std::env::var_os(REFRESH_POSTCHECK_WITNESS_ENVIRONMENT) else {
        return Ok(());
    };
    std::fs::write(Path::new(&path), EMPTY_MARKER).map_err(Into::into)
}

fn capture_barrier(request: &str, acknowledgement: &str) -> Option<BarrierHook> {
    Some(BarrierHook {
        request: std::env::var_os(request)?,
        acknowledgement: std::env::var_os(acknowledgement)?,
    })
}

fn forward_barrier(
    command: &mut Command,
    hook: Option<BarrierHook>,
    request: &str,
    acknowledgement: &str,
) {
    if let Some(hook) = hook {
        command
            .env(request, hook.request)
            .env(acknowledgement, hook.acknowledgement);
    }
}

fn run_barrier(
    hook: Option<BarrierHook>,
    used: &AtomicBool,
    timeout_detail: &'static str,
) -> anyhow::Result<bool> {
    let Some(hook) = hook else {
        return Ok(false);
    };
    if used.swap(true, Ordering::AcqRel) {
        return Ok(false);
    }
    std::fs::write(Path::new(&hook.request), EMPTY_MARKER)?;
    let deadline = Instant::now() + WATCHDOG_SETTLEMENT_TIMEOUT;
    while !Path::new(&hook.acknowledgement).exists() {
        anyhow::ensure!(Instant::now() < deadline, timeout_detail);
        std::thread::sleep(FORCE_SIGNAL_HOOK_POLL_INTERVAL);
    }
    Ok(true)
}

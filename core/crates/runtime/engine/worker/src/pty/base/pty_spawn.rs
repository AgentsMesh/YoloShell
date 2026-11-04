#[cfg(test)]
use portable_pty::Child;
use portable_pty::SlavePty;

pub(crate) const STARTUP_BIND_CONTEXT: &str = "bind spawned PTY root to its wait and process identities";
pub(crate) const STARTUP_IDENTITY_PANIC_DETAIL: &str = "capture spawned PTY root identity panicked";
pub(crate) const SPAWN_GUARD_RELEASE_PANIC_DETAIL: &str = "release PTY spawn guard panicked";

#[path = "pty_spawn_failure.rs"]
pub(crate) mod pty_spawn_failure;

#[cfg(test)]
pub(crate) fn spawn_waitable_pty_root(
    slave: &dyn SlavePty,
    command: CommandBuilder,
) -> Result<Box<dyn Child + Send + Sync>> {
    let _sigchld_guard = process::waitable_child_spawn_guard()?;
    slave.spawn_command(command)
}

pub(crate) fn spawn_tracked_pty_root(
    slave: &dyn SlavePty,
    command: BootstrapSpawnCommand,
    proof: process::PtySessionProof,
    output_progress: ExclusivePtyOutput,
    cleanup_scope: &mut StartupCleanupScope,
) -> Result<StartupChildGuard> {
    let BootstrapSpawnCommand { command, gate } = command;
    let sigchld_guard = process::waitable_child_spawn_guard()?;
    let spawn_boundary = sigchld_guard.boundary();
    let mut guard = spawn_with_reserved_reaper(
        cleanup_scope,
        || slave.spawn_command(command),
        &spawn_boundary,
        output_progress,
        gate,
    )?;
    drop(spawn_boundary);
    bind_before_releasing_spawn_guard(sigchld_guard, |spawn_guard| {
        guard.bind_pty(proof, &spawn_guard.boundary())
    })
    .context(STARTUP_BIND_CONTEXT)?;
    Ok(guard)
}

pub(crate) fn bind_before_releasing_spawn_guard<SpawnGuard, Value>(
    spawn_guard: SpawnGuard,
    bind: impl FnOnce(&SpawnGuard) -> Result<Value>,
) -> Result<Value> {
    let binding = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| bind(&spawn_guard)));
    let release = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(spawn_guard)));
    match (binding, release) {
        (Ok(binding), Ok(())) => binding,
        (Err(payload), Ok(())) => std::panic::resume_unwind(payload),
        (Err(payload), Err(release_payload)) => {
            let _ = supervision::panic_detail(release_payload);
            std::panic::resume_unwind(payload)
        }
        (Ok(binding), Err(payload)) => {
            let detail = supervision::panic_detail(payload);
            let _ = supervision::contain(|| drop(binding));
            Err(anyhow::anyhow!(
                "{SPAWN_GUARD_RELEASE_PANIC_DETAIL}: {detail}"
            ))
        }
    }
}

pub(crate) fn spawn_with_reserved_reaper(
    cleanup_scope: &mut StartupCleanupScope,
    spawn: impl FnOnce() -> Result<process::OwnedPtyChild>,
    boundary: &process::WaitableChildSpawnBoundary<'_>,
    output_progress: ExclusivePtyOutput,
    gate: UnreleasedBootstrapGate,
) -> Result<StartupChildGuard> {
    process::signal_authority::require_pty_spawn_signal_capability()?;
    let fault_reaper = pty_fault_reaper::FaultReaperReservation::reserve()?;
    let reservation =
        startup_untracked_reaper::StartupUntrackedReaperReservation::reserve(cleanup_scope)?;
    let child = spawn()?;
    let identity = match capture_spawned_identity(&child) {
        Ok(identity) => identity,
        Err(error) => {
            let child = process::retain_unidentified_child(boundary, child);
            return Err(pty_spawn_failure::settle(child, error));
        }
    };
    Ok(StartupChildGuard::with_reserved_output(
        child,
        identity,
        reservation,
        fault_reaper,
        output_progress,
        gate,
    ))
}

pub(crate) fn capture_spawned_identity(
    child: &process::OwnedPtyChild,
) -> Result<process::DirectChildIdentity> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let identity = process::DirectChildIdentity::capture_candidate(child)?;
        identity.prove_current(child)?;
        Ok::<_, anyhow::Error>(identity)
    })) {
        Ok(identity) => identity,
        Err(payload) => {
            let detail = supervision::panic_detail(payload);
            Err(anyhow::anyhow!("{STARTUP_IDENTITY_PANIC_DETAIL}: {detail}"))
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))] #[path = "../../../integration_tests/pty/spawn/pty_spawn_capability_tests.rs"]
mod capability_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))] #[path = "../../../integration_tests/pty/spawn/pty_spawn_emergency_reservation_tests.rs"]
mod emergency_reservation_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))] #[path = "../../../integration_tests/pty/spawn/pty_spawn_identity_failure_tests.rs"]
mod identity_failure_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))] #[path = "../../../integration_tests/pty/spawn/pty_spawn_reservation_tests.rs"]
mod reservation_tests;

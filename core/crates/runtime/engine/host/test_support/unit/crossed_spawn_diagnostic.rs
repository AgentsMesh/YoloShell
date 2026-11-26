use super::*;
use crate::host::tests::*;

#[path = "crossed_spawn_diagnostic_constants.rs"]
mod diagnostic_constants;
#[path = "crossed_spawn_diagnostic_read.rs"]
mod observation;
use diagnostic_constants::*;
use observation::{Evidence, Reading, read};

#[derive(Debug)]
pub(super) struct Diagnostic {
    call_elapsed: Duration,
    retained_ledger: Ledger,
    disk_ledger: Evidence<Ledger>,
    catalog_state: RecoveryState,
    manifest: Evidence<RecoveryState>,
    launch: Evidence<Launch>,
    runtime: Evidence<Runtime>,
    checkpoint: Evidence<Checkpoint>,
}

#[derive(Debug)]
struct Ledger {
    phase: CreationPhase,
    has_launch_plan: bool,
    launch_generation: u64,
    matches_retained: bool,
}

#[derive(Debug)]
pub(super) struct Launch {
    matches_plan: Option<bool>,
    matches_controller: bool,
}

#[derive(Debug)]
struct Runtime {
    matches_plan: Option<bool>,
    delivery: BootstrapDelivery,
    worker_pid: u32,
    child_pid: u32,
}

#[derive(Debug)]
struct Checkpoint {
    matches_planned_ids: Option<bool>,
    stage: crate::worker::StartupStage,
    phase: crate::worker::StartupPhase,
    status: crate::worker::StartupStatus,
    worker_pid: u32,
    child_pid: Option<u32>,
}

/// Failure-only typed evidence; this cannot prove that the OS spawned.
pub(super) fn capture(
    host: &Host,
    request: &CreateTerminal,
    catalog: &TerminalManifest,
    elapsed: Duration,
) -> Diagnostic {
    let retained = baseline(host, request, catalog);
    let session = retained.result.terminal_id;
    let plan = retained.launch.as_ref();
    Diagnostic {
        call_elapsed: elapsed,
        retained_ledger: ledger(&retained, &retained),
        disk_ledger: read(
            &host.paths.home().join(CREATION_LEDGER_FILE_NAME),
            MAX_CREATION_LEDGER_BYTES,
            |stored: CreationLedger| {
                if stored.validate().is_err() {
                    return Reading::InvalidRecord;
                }
                let mut matching = stored.bindings.iter().filter(|binding| {
                    binding.creation_token_digest == retained.creation_token_digest
                });
                let Some(binding) = matching.next() else {
                    return Reading::MismatchedBinding;
                };
                if matching.next().is_some()
                    || binding.result.terminal_id != session
                    || binding.spec.request_fingerprint != retained.spec.request_fingerprint
                {
                    return Reading::MismatchedBinding;
                }
                Reading::Present(ledger(binding, &retained))
            },
        ),
        catalog_state: catalog.recovery_state,
        manifest: read(
            &host.paths.manifest(session),
            MAX_CREATION_EVIDENCE_BYTES,
            |value: TerminalManifest| {
                if value.terminal_id != session {
                    return Reading::MismatchedBinding;
                }
                Reading::Present(value.recovery_state)
            },
        ),
        launch: read(
            &host.paths.worker_launch(session),
            MAX_CREATION_EVIDENCE_BYTES,
            |value: WorkerLaunchSpec| {
                if value.terminal_id != session {
                    return Reading::MismatchedBinding;
                }
                Reading::Present(Launch {
                    matches_plan: plan.map(|plan| launch_matches_identity(&value, plan)),
                    matches_controller: value.controller_generation == host.controller_generation,
                })
            },
        ),
        runtime: read(
            &host.paths.runtime_record(session),
            MAX_CREATION_EVIDENCE_BYTES,
            |value: RuntimeRecord| {
                if value.terminal_id != session {
                    return Reading::MismatchedBinding;
                }
                Reading::Present(Runtime {
                    matches_plan: plan.map(|plan| runtime_matches_identity(&value, plan)),
                    delivery: value.bootstrap_delivery,
                    worker_pid: value.worker_pid,
                    child_pid: value.child_pid,
                })
            },
        ),
        checkpoint: read(
            &host.paths.worker_startup_checkpoint(session),
            crate::paths::constants::WORKER_STARTUP_CHECKPOINT_MAX_BYTES,
            |value: crate::worker::StartupCheckpointEvidence| {
                if value.terminal_id != session {
                    return Reading::MismatchedBinding;
                }
                Reading::Present(Checkpoint {
                    matches_planned_ids: plan.map(|plan| {
                        value.runtime_epoch == plan.runtime_epoch
                            && value.worker_id == plan.worker_id
                    }),
                    stage: value.stage,
                    phase: value.phase,
                    status: value.status,
                    worker_pid: value.worker.pid,
                    child_pid: value.child.map(|child| child.pid),
                })
            },
        ),
    }
}

fn baseline(host: &Host, request: &CreateTerminal, catalog: &TerminalManifest) -> CreationBinding {
    let digest = creation_token_digest(request.creation_token);
    let fingerprint =
        CreationSpec::fingerprint(request).unwrap_or_else(|_| panic!("{BASELINE_FINGERPRINT}"));
    let retained = host.creation_capabilities.lock().expect(BASELINE_LOCK);
    let mut matching = retained
        .bindings
        .iter()
        .filter(|binding| binding.creation_token_digest == digest);
    let binding = matching.next().expect(BASELINE_MISSING);
    assert!(matching.next().is_none(), "{BASELINE_AMBIGUOUS}");
    assert!(
        binding.spec.request_fingerprint == fingerprint
            && binding.result.terminal_id == catalog.terminal_id,
        "{BASELINE_MISMATCH}"
    );
    binding.clone()
}

pub(super) fn ledger(value: &CreationBinding, retained: &CreationBinding) -> Ledger {
    Ledger {
        phase: value.phase,
        has_launch_plan: value.launch.is_some(),
        launch_generation: value.launch_generation,
        matches_retained: value == retained,
    }
}

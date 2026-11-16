use super::{ARMED_DETAIL, GuardianState, PROCESS_DETAIL, RUNTIME_DETAIL};
use crate::custody_guardian::settlement_runtime_file::RetainedRuntimeFile;
use crate::model::RuntimeRecord;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::{
    CustodyDurableRecord, CustodyGuardianBinding, CustodyHolderIdentity, CustodyHolderObservation,
};
use anyhow::{Context, Result, ensure};
use yolo_paths::durable_mutation::SessionDurableMutationGuard;

const RUNTIME_GENERATION_DETAIL: &str =
    "detached guardian armed runtime controller generation changed unexpectedly";
const RUNTIME_SAME_VALUE_DETAIL: &str =
    "detached guardian armed runtime same-value identity changed unexpectedly";
const RUNTIME_REPLACEMENT_DETAIL: &str =
    "detached guardian armed runtime replacement reused the retained file object";

pub(super) enum Admission {
    AwaitingActivation,
    Activated(RuntimeRecord, FileIdentity, Option<RetainedRuntimeFile>),
    Continue,
    Restore,
}

pub(super) fn evaluate(
    namespace: &NamespaceLease,
    paths: &Paths,
    state: &GuardianState,
    binding: CustodyGuardianBinding,
    record: &CustodyDurableRecord,
    generation: u64,
) -> Result<Admission> {
    match binding.host_holder().observation() {
        CustodyHolderObservation::Current => {
            return match activated_runtime(namespace, paths, state, generation)? {
                Some((runtime, identity, file)) => {
                    Ok(Admission::Activated(runtime, identity, file))
                }
                None => Ok(Admission::AwaitingActivation),
            };
        }
        CustodyHolderObservation::Indeterminate => anyhow::bail!(PROCESS_DETAIL),
        CustodyHolderObservation::ProvenGone => {}
    }
    match worker(record).observation() {
        CustodyHolderObservation::Current => Ok(Admission::Restore),
        CustodyHolderObservation::ProvenGone => Ok(Admission::Continue),
        CustodyHolderObservation::Indeterminate => anyhow::bail!(PROCESS_DETAIL),
    }
}

fn activated_runtime(
    namespace: &NamespaceLease,
    paths: &Paths,
    state: &GuardianState,
    generation: u64,
) -> Result<Option<(RuntimeRecord, FileIdentity, Option<RetainedRuntimeFile>)>> {
    let (runtime, runtime_identity) = state.retained_runtime()?;
    let (current, current_identity): (RuntimeRecord, FileIdentity) =
        namespace.read_json_with_identity(&paths.runtime_record(runtime.terminal_id))?;
    let activated = runtime_activated(
        paths,
        runtime,
        runtime_identity,
        &current,
        current_identity,
        generation,
    )?;
    if !activated {
        return Ok(None);
    }
    let file = state
        .runtime_replacement_required(&current, current_identity)?
        .then(|| RetainedRuntimeFile::capture(namespace, paths, &current, current_identity))
        .transpose()?;
    Ok(Some((current, current_identity, file)))
}

pub(super) fn runtime_activated(
    paths: &Paths,
    retained: &RuntimeRecord,
    retained_identity: FileIdentity,
    current: &RuntimeRecord,
    current_identity: FileIdentity,
    generation: u64,
) -> Result<bool> {
    current
        .validate_update_from(retained)
        .context(RUNTIME_DETAIL)?;
    ensure!(
        generation >= retained.controller_generation
            && matches!(
                current.controller_generation,
                value if value == retained.controller_generation || value == generation
            ),
        RUNTIME_GENERATION_DETAIL
    );
    if current == retained {
        ensure!(
            current_identity == retained_identity,
            RUNTIME_SAME_VALUE_DETAIL
        );
    } else {
        ensure!(
            !current_identity.same_object(retained_identity),
            RUNTIME_REPLACEMENT_DETAIL
        );
    }
    if current.controller_generation != generation || !current.bootstrap_delivery.is_serviceable() {
        return Ok(false);
    }
    crate::worker::worker_startup_checkpoint_allows_lifecycle_mutation(
        paths,
        current.terminal_id,
        Some(current),
    )
}

fn worker(record: &CustodyDurableRecord) -> CustodyHolderIdentity {
    CustodyHolderIdentity {
        pid: record.runtime.worker_pid,
        start_time: record.runtime.worker_start_time,
    }
}

pub(super) fn require_armed(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    armed: &CustodyDurableRecord,
    identity: FileIdentity,
) -> Result<()> {
    guard.require_no_pending_intent()?;
    let path = paths.pty_custody(armed.runtime.terminal_id);
    let current = guard.namespace().read_json_with_identity(&path)?;
    ensure!(current == (armed.clone(), identity), ARMED_DETAIL);
    guard.require_no_pending_intent()
}

pub(super) fn require_runtime(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    runtime: &RuntimeRecord,
    identity: FileIdentity,
) -> Result<()> {
    ensure!(
        read_runtime(guard, paths, runtime)? == (runtime.clone(), identity),
        RUNTIME_DETAIL
    );
    Ok(())
}

pub(super) fn read_runtime(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    runtime: &RuntimeRecord,
) -> Result<(RuntimeRecord, FileIdentity)> {
    guard
        .namespace()
        .read_json_with_identity(&paths.runtime_record(runtime.terminal_id))
}

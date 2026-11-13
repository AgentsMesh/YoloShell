use super::settlement::GuardianState;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::CustodyDurableRecord;
use anyhow::{Context, Result, ensure};
use yolo_paths::durable_mutation::SessionDurableMutationGuard;

#[path = "settlement_armed_admission.rs"]
mod admission;
#[path = "settlement_armed_restoration.rs"]
mod restoration;

const ARMED_DETAIL: &str = "detached guardian armed custody changed unexpectedly";
const PROCESS_DETAIL: &str = "detached guardian armed process observation is indeterminate";
const RUNTIME_DETAIL: &str = "detached guardian armed runtime changed unexpectedly";
const RECONCILIATION_DETAIL: &str = "reconcile restored detached guardian preparation";

pub(super) enum Resolution {
    AwaitingActivation,
    Continue,
    Restored(CustodyDurableRecord, FileIdentity),
}

pub(super) fn resolve(
    namespace: &NamespaceLease,
    paths: &Paths,
    state: &mut GuardianState,
    armed: &CustodyDurableRecord,
    armed_identity: FileIdentity,
) -> Result<Resolution> {
    let Some(prepared) = state.pending_record().cloned() else {
        return Ok(Resolution::Continue);
    };
    let candidate = prepared
        .prepared_guardian_candidate()
        .context(ARMED_DETAIL)?;
    let binding = candidate.guardian.context(ARMED_DETAIL)?;
    let preparation = state.preparation_for(binding)?;
    let expected_armed = prepared
        .arm_exact_prepared_guardian(binding)
        .context(ARMED_DETAIL)?;
    ensure!(expected_armed == *armed, ARMED_DETAIL);
    match admission::evaluate(
        namespace,
        paths,
        state,
        binding,
        armed,
        candidate.controller_generation,
    )? {
        admission::Admission::AwaitingActivation => {
            return Ok(Resolution::AwaitingActivation);
        }
        admission::Admission::Activated(_, _, _) => {}
        admission::Admission::Continue => return Ok(Resolution::Continue),
        admission::Admission::Restore => {}
    }
    let guard = SessionDurableMutationGuard::acquire_participant(
        paths,
        namespace,
        armed.runtime.terminal_id,
    )?;
    admission::require_armed(&guard, paths, armed, armed_identity)?;
    match admission::evaluate(
        namespace,
        paths,
        state,
        binding,
        armed,
        candidate.controller_generation,
    )? {
        admission::Admission::AwaitingActivation => {
            return Ok(Resolution::AwaitingActivation);
        }
        admission::Admission::Activated(runtime, identity, file) => {
            state.adopt_activated_runtime(runtime, identity, file)?;
            return Ok(Resolution::Continue);
        }
        admission::Admission::Continue => return Ok(Resolution::Continue),
        admission::Admission::Restore => {}
    }
    let (runtime, runtime_identity) = state.retained_runtime()?;
    let runtime = runtime.clone();
    let current = admission::read_runtime(&guard, paths, &runtime)?;
    if current != (runtime.clone(), runtime_identity) {
        ensure!(
            admission::runtime_activated(
                paths,
                &runtime,
                runtime_identity,
                &current.0,
                current.1,
                candidate.controller_generation,
            )?,
            RUNTIME_DETAIL
        );
        let file = super::settlement_runtime_file::RetainedRuntimeFile::capture(
            namespace, paths, &current.0, current.1,
        )?;
        state.adopt_activated_runtime(current.0, current.1, Some(file))?;
        return Ok(Resolution::Continue);
    }
    let identity = restoration::restore(
        &guard,
        paths,
        armed,
        armed_identity,
        &prepared,
        &runtime,
        runtime_identity,
    )?;
    state.restore_pending(identity)?;
    ensure!(
        state.pending() == Some(preparation.with_custody_file_identity(identity)),
        ARMED_DETAIL
    );
    Ok(Resolution::Restored(prepared, identity))
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/settlement/settlement_armed_activation_tests.rs"]
mod activation_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/settlement/settlement_armed_progress_tests.rs"]
mod progress_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/settlement/settlement_retirement_tests.rs"]
mod retirement_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../test_support/settlement_test_custody.rs"]
pub(super) mod test_custody;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../test_support/replacement_test_support.rs"]
pub(super) mod test_support;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/settlement/settlement_armed_tests.rs"]
mod tests;

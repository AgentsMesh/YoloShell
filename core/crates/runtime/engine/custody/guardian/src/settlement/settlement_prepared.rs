use super::settlement::GuardianState;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::{
    CustodyDurableRecord, CustodyDurableState, CustodyGuardianBinding, CustodyHolderObservation,
};
use anyhow::{Context, Result, ensure};

#[path = "settlement_prepared_replacement.rs"]
mod replacement;

const PREPARED_DETAIL: &str = "detached guardian preparation changed unexpectedly";
const PROCESS_DETAIL: &str = "detached guardian replacement process is not proven gone";

pub(super) enum Resolution {
    Pending,
    InitialOrphan,
    Restored(CustodyDurableRecord, FileIdentity),
    Armed(CustodyDurableRecord, FileIdentity),
}

pub(super) fn resolve(
    namespace: &NamespaceLease,
    paths: &Paths,
    state: &mut GuardianState,
    prepared: &CustodyDurableRecord,
    prepared_identity: FileIdentity,
) -> Result<Resolution> {
    let candidate = prepared
        .prepared_guardian_candidate()
        .context(PREPARED_DETAIL)?;
    let binding = candidate.guardian.context(PREPARED_DETAIL)?;
    let preparation = state.preparation_for(binding)?;
    ensure!(
        preparation.custody_file_identity() == prepared_identity,
        PREPARED_DETAIL
    );
    match binding.host_holder().observation() {
        CustodyHolderObservation::Current => return Ok(Resolution::Pending),
        CustodyHolderObservation::Indeterminate => anyhow::bail!(PROCESS_DETAIL),
        CustodyHolderObservation::ProvenGone => {}
    }
    if let Some(resolution) = replacement::resolve(namespace, paths, state, prepared, preparation)?
    {
        return Ok(resolution);
    }
    let predecessor = prepared
        .abandon_prepared_guardian(binding)
        .context(PREPARED_DETAIL)?;
    if matches!(predecessor.state, CustodyDurableState::Pending {}) {
        return Ok(Resolution::InitialOrphan);
    }
    match super::preparation::abandon(namespace, paths, prepared, preparation) {
        Ok(installed) => resolve_restored(state, binding, predecessor, installed),
        Err(original) => resolve_race(namespace, paths, state, prepared, predecessor, binding)
            .map_err(|race| original.context(format!("{PREPARED_DETAIL}: {race:#}"))),
    }
}

fn resolve_race(
    namespace: &NamespaceLease,
    paths: &Paths,
    state: &mut GuardianState,
    prepared: &CustodyDurableRecord,
    predecessor: CustodyDurableRecord,
    binding: CustodyGuardianBinding,
) -> Result<Resolution> {
    let (current, identity): (CustodyDurableRecord, FileIdentity) =
        namespace.read_json_with_identity(&paths.pty_custody(prepared.runtime.terminal_id))?;
    if current == predecessor {
        return resolve_restored(state, binding, predecessor, (current, identity));
    }
    let candidate = prepared
        .prepared_guardian_candidate()
        .context(PREPARED_DETAIL)?;
    let armed = current.armed_lease() == Some(candidate);
    ensure!(armed, PREPARED_DETAIL);
    state.adopt_prepared_binding(binding)?;
    Ok(Resolution::Armed(current, identity))
}

fn resolve_restored(
    state: &mut GuardianState,
    binding: CustodyGuardianBinding,
    expected: CustodyDurableRecord,
    installed: (CustodyDurableRecord, FileIdentity),
) -> Result<Resolution> {
    ensure!(installed.0 == expected, PREPARED_DETAIL);
    match installed.0.state {
        CustodyDurableState::Pending {} => anyhow::bail!(PREPARED_DETAIL),
        CustodyDurableState::Armed { .. } | CustodyDurableState::Retired { .. } => {
            state.abandon_successor(binding)?;
            Ok(Resolution::Restored(installed.0, installed.1))
        }
        CustodyDurableState::PreparedGuardian { .. } => anyhow::bail!(PREPARED_DETAIL),
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/settlement/settlement_prepared_tests.rs"]
mod tests;

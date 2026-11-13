use super::{PREPARED_DETAIL, PROCESS_DETAIL, Resolution};
use crate::custody_guardian::settlement::GuardianState;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::{
    CustodyDurableRecord, CustodyDurableState, CustodyGuardianBinding, CustodyGuardianPreparation,
    CustodyHolderIdentity, CustodyHolderObservation,
};
use anyhow::{Result, ensure};

pub(super) fn resolve(
    namespace: &NamespaceLease,
    paths: &Paths,
    state: &mut GuardianState,
    prepared: &CustodyDurableRecord,
    preparation: CustodyGuardianPreparation,
) -> Result<Option<Resolution>> {
    let Some((guardian, holder)) = predecessor(prepared, preparation.binding())? else {
        return Ok(None);
    };
    ensure!(
        guardian.process().observation() == CustodyHolderObservation::ProvenGone
            && holder.observation() == CustodyHolderObservation::ProvenGone,
        PROCESS_DETAIL
    );
    match worker(prepared).observation() {
        CustodyHolderObservation::Current => return Ok(Some(Resolution::Pending)),
        CustodyHolderObservation::Indeterminate => anyhow::bail!(PROCESS_DETAIL),
        CustodyHolderObservation::ProvenGone => {}
    }
    let resolution = match super::super::preparation::arm(namespace, paths, prepared, preparation) {
        Ok(installed) => resolve_armed(state, prepared, preparation.binding(), installed),
        Err(original) => resolve_race(namespace, paths, state, prepared, preparation.binding())
            .map_err(|race| original.context(format!("{PREPARED_DETAIL}: {race:#}"))),
    }?;
    Ok(Some(resolution))
}

fn predecessor(
    prepared: &CustodyDurableRecord,
    candidate: CustodyGuardianBinding,
) -> Result<Option<(CustodyGuardianBinding, CustodyHolderIdentity)>> {
    let CustodyDurableState::PreparedGuardian { predecessor, .. } = &prepared.state else {
        anyhow::bail!(PREPARED_DETAIL)
    };
    let CustodyDurableState::Armed {
        holder,
        guardian: Some(guardian),
        ..
    } = predecessor.as_ref()
    else {
        ensure!(
            matches!(predecessor.as_ref(), CustodyDurableState::Pending {}),
            PREPARED_DETAIL
        );
        return Ok(None);
    };
    let same_guardian = guardian.guardian_id() == candidate.guardian_id()
        && guardian.process() == candidate.process();
    Ok((!same_guardian).then_some((*guardian, *holder)))
}

fn worker(prepared: &CustodyDurableRecord) -> CustodyHolderIdentity {
    CustodyHolderIdentity {
        pid: prepared.runtime.worker_pid,
        start_time: prepared.runtime.worker_start_time,
    }
}

fn resolve_race(
    namespace: &NamespaceLease,
    paths: &Paths,
    state: &mut GuardianState,
    prepared: &CustodyDurableRecord,
    binding: CustodyGuardianBinding,
) -> Result<Resolution> {
    let installed =
        namespace.read_json_with_identity(&paths.pty_custody(prepared.runtime.terminal_id))?;
    resolve_armed(state, prepared, binding, installed)
}

fn resolve_armed(
    state: &mut GuardianState,
    prepared: &CustodyDurableRecord,
    binding: CustodyGuardianBinding,
    installed: (CustodyDurableRecord, FileIdentity),
) -> Result<Resolution> {
    ensure!(
        installed.0.armed_lease() == prepared.prepared_guardian_candidate(),
        PREPARED_DETAIL
    );
    state.adopt_prepared_binding(binding)?;
    Ok(Resolution::Armed(installed.0, installed.1))
}

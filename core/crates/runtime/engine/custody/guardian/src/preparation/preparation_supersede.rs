use super::PREPARATION_DETAIL;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::{
    CustodyDurableRecord, CustodyGuardianBinding, CustodyGuardianPreparation,
    CustodyHolderObservation, CustodyOffer, CustodyRequest,
};
use anyhow::{Context, Result, ensure};

pub(super) enum Plan {
    Exact(CustodyGuardianPreparation),
    Replace(CustodyDurableRecord),
}

pub(super) fn plan(
    current: &CustodyDurableRecord,
    identity: FileIdentity,
    request: &CustodyRequest,
    offer: &CustodyOffer,
    binding: CustodyGuardianBinding,
    retained: Option<CustodyGuardianPreparation>,
) -> Result<Plan> {
    ensure!(binding.process().matches(), PREPARATION_DETAIL);
    if exact_retry(current, identity, request, offer, binding, retained) {
        return retained.map(Plan::Exact).context(PREPARATION_DETAIL);
    }
    if let Some(prepared) = current.prepare_guardian(request, offer, binding) {
        ensure!(retained.is_none(), PREPARATION_DETAIL);
        return Ok(Plan::Replace(prepared));
    }
    let candidate = current
        .prepared_guardian_candidate()
        .context(PREPARATION_DETAIL)?;
    let predecessor = candidate.guardian.context(PREPARATION_DETAIL)?;
    ensure!(
        candidate.holder.observation() == CustodyHolderObservation::ProvenGone,
        PREPARATION_DETAIL
    );
    match retained {
        Some(retained) => ensure!(
            retained.binding() == predecessor
                && retained.custody_file_identity() == identity
                && same_guardian(predecessor, binding)
                && predecessor.process().observation() == CustodyHolderObservation::Current,
            PREPARATION_DETAIL
        ),
        None => ensure!(
            predecessor.process().observation() == CustodyHolderObservation::ProvenGone,
            PREPARATION_DETAIL
        ),
    }
    current
        .supersede_prepared_guardian(request, offer, binding)
        .map(Plan::Replace)
        .context(PREPARATION_DETAIL)
}

pub(super) fn exact_retry(
    record: &CustodyDurableRecord,
    identity: FileIdentity,
    request: &CustodyRequest,
    offer: &CustodyOffer,
    binding: CustodyGuardianBinding,
    retained: Option<CustodyGuardianPreparation>,
) -> bool {
    retained.is_some_and(|preparation| {
        preparation.binding() == binding && preparation.custody_file_identity() == identity
    }) && record.is_valid()
        && record.runtime == request.runtime
        && binding.matches(request, offer)
        && record
            .prepared_guardian_candidate()
            .is_some_and(|candidate| binding.matches_lease_identity(&candidate))
}

fn same_guardian(first: CustodyGuardianBinding, second: CustodyGuardianBinding) -> bool {
    first.guardian_id() == second.guardian_id() && first.process() == second.process()
}

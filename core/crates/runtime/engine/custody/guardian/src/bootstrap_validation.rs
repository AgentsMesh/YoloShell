use super::protocol::GuardianBootstrap;
use crate::model::RuntimeRecord;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::{CustodyDurableRecord, CustodyHolderObservation, CustodyProfile};
use anyhow::{Context, Result, ensure};

const BOOTSTRAP_DETAIL: &str = "detached PTY custody guardian bootstrap is not exact";

pub(super) fn validate(
    bootstrap: &GuardianBootstrap,
    runtime: &RuntimeRecord,
    runtime_identity: FileIdentity,
    durable: &CustodyDurableRecord,
    durable_identity: FileIdentity,
) -> Result<()> {
    ensure!(bootstrap.is_well_formed(), BOOTSTRAP_DETAIL);
    ensure!(
        bootstrap.request.runtime.matches_record(runtime)
            && bootstrap.request.auth_token() == runtime.auth_token
            && bootstrap.runtime_file_identity == runtime_identity
            && bootstrap.request.holder().matches(),
        BOOTSTRAP_DETAIL
    );
    ensure!(
        bootstrap.durable_predecessor.record() == durable
            && bootstrap.durable_predecessor.file_identity() == durable_identity
            && durable.is_valid()
            && durable.runtime == bootstrap.request.runtime
            && bootstrap.offer.profile.is_active(),
        BOOTSTRAP_DETAIL
    );
    match &durable.state {
        crate::pty_custody::CustodyDurableState::Armed { .. } => {
            let predecessor = durable.armed_lease().context(BOOTSTRAP_DETAIL)?;
            validate_successor(bootstrap, runtime, predecessor)?
        }
        crate::pty_custody::CustodyDurableState::PreparedGuardian { .. } => {
            validate_prepared_successor(bootstrap, runtime, durable)?
        }
        crate::pty_custody::CustodyDurableState::Pending {} => ensure!(
            bootstrap.request.controller_generation == runtime.controller_generation,
            BOOTSTRAP_DETAIL
        ),
        crate::pty_custody::CustodyDurableState::Retired { .. } => {
            anyhow::bail!(BOOTSTRAP_DETAIL)
        }
    }
    Ok(())
}

fn validate_prepared_successor(
    bootstrap: &GuardianBootstrap,
    runtime: &RuntimeRecord,
    durable: &CustodyDurableRecord,
) -> Result<()> {
    let predecessor = durable
        .prepared_guardian_predecessor_lease()
        .ok_or_else(|| anyhow::anyhow!(BOOTSTRAP_DETAIL))?;
    validate_successor(bootstrap, runtime, predecessor)?;
    let candidate = durable
        .prepared_guardian_candidate()
        .ok_or_else(|| anyhow::anyhow!(BOOTSTRAP_DETAIL))?;
    let guardian_gone = candidate.guardian.is_some_and(|guardian| {
        guardian.matches_durable_lease(&bootstrap.request.runtime, &candidate)
            && guardian.process().observation() == CustodyHolderObservation::ProvenGone
    });
    ensure!(
        guardian_gone
            && candidate.holder.observation() == CustodyHolderObservation::ProvenGone
            && bootstrap.request.controller_generation > candidate.controller_generation
            && bootstrap.request.lease_id != candidate.lease_id
            && bootstrap.offer.offer_id != candidate.offer_id
            && bootstrap.offer.descriptor_set == candidate.descriptor_set
            && bootstrap.offer.recovery_identity_sha256 == candidate.recovery_identity_sha256,
        BOOTSTRAP_DETAIL
    );
    Ok(())
}

fn validate_successor(
    bootstrap: &GuardianBootstrap,
    runtime: &RuntimeRecord,
    predecessor: crate::pty_custody::CustodyDurableLease,
) -> Result<()> {
    let guardian_gone = predecessor.guardian.is_some_and(|binding| {
        binding.process().observation() == CustodyHolderObservation::ProvenGone
            && binding.matches_durable_lease(&bootstrap.request.runtime, &predecessor)
    });
    let profile_valid = matches!(
        (predecessor.profile, bootstrap.offer.profile),
        (CustodyProfile::ActiveRunning, CustodyProfile::ActiveRunning)
            | (
                CustodyProfile::ActiveRunning,
                CustodyProfile::ActiveTerminalReadOnly
            )
            | (
                CustodyProfile::ActiveTerminalReadOnly,
                CustodyProfile::ActiveTerminalReadOnly
            )
    );
    ensure!(
        guardian_gone
            && predecessor.holder.observation() == CustodyHolderObservation::ProvenGone
            && bootstrap.request.controller_generation > predecessor.controller_generation
            && bootstrap.request.controller_generation > runtime.controller_generation
            && bootstrap.request.lease_id != predecessor.lease_id
            && bootstrap.offer.offer_id != predecessor.offer_id
            && bootstrap.offer.descriptor_set == predecessor.descriptor_set
            && bootstrap.offer.recovery_identity_sha256 == predecessor.recovery_identity_sha256
            && profile_valid,
        BOOTSTRAP_DETAIL
    );
    Ok(())
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/preparation/replacement_tests.rs"]
mod tests;

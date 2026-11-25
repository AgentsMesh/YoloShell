use super::*;
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyDurableLease, CustodyDurableRecord, CustodyDurableSnapshot,
    CustodyDurableState, CustodyHolderObservation, CustodyOffer, CustodyProfile, CustodyRequest,
};

#[path = "secondary_custody_validation_prepared.rs"]
mod prepared;

const DURABLE_READ_CONTEXT: &str = "read durable PTY custody record";
const DURABLE_IDENTITY_DETAIL: &str = "durable PTY custody record does not match its runtime";
const DURABLE_COMMIT_DETAIL: &str = "durable PTY custody commit does not match its offer";
const HOLDER_IDENTITY_DETAIL: &str = "Host PTY custody holder identity is no longer exact";
const LEASE_IDENTITY_DETAIL: &str = "installed PTY custody lease identity changed";
const OFFER_IDENTITY_DETAIL: &str = "PTY custody offer does not match its request";
const OFFER_PROFILE_DETAIL: &str = "PTY custody offer has the wrong platform descriptor profile";
const ACTIVE_PREDECESSOR_DETAIL: &str =
    "active macOS PTY custody offer has no reusable durable predecessor";
const RELEASED_SUCCESSOR_EVIDENCE_DETAIL: &str =
    "released PTY successor lacks exact sole-holder exit evidence";
const RUNTIME_IDENTITY_DETAIL: &str = "PTY custody runtime identity is incomplete";
const RUNTIME_PUBLICATION_DETAIL: &str = "PTY custody runtime publication changed during handshake";

pub(super) fn validate_runtime(
    host: &Host,
    record: &RuntimeRecord,
    runtime: &CustodyRuntimeIdentity,
) -> Result<(), WorkerRpcError> {
    identity(runtime.is_complete(), RUNTIME_IDENTITY_DETAIL)?;
    identity(runtime.matches_record(record), RUNTIME_IDENTITY_DETAIL)?;
    identity(
        host.secondary_custody_holder.matches(),
        HOLDER_IDENTITY_DETAIL,
    )
}

pub(super) fn validate_lease(
    host: &Host,
    record: &RuntimeRecord,
    lease: &SecondaryCustodyLease,
) -> Result<(), WorkerRpcError> {
    identity(
        lease.matches(
            record,
            host.secondary_custody_holder,
            host.controller_generation,
        ),
        LEASE_IDENTITY_DETAIL,
    )
}

pub(super) fn validate_offer(
    request: &CustodyRequest,
    offer: &CustodyOffer,
    predecessor: &CustodyDurableRecord,
) -> Result<(), WorkerRpcError> {
    identity(offer.matches_request(request), OFFER_IDENTITY_DETAIL)?;
    match offer.profile {
        CustodyProfile::ActiveRunning | CustodyProfile::ActiveTerminalReadOnly => {
            identity(
                platform::accepts_descriptor_set(offer.descriptor_set),
                OFFER_PROFILE_DETAIL,
            )?;
            validate_active_predecessor(offer, predecessor)
        }
        CustodyProfile::ReleasedTerminal => validate_released_offer(request, offer, predecessor),
    }
}

fn validate_active_predecessor(
    offer: &CustodyOffer,
    predecessor: &CustodyDurableRecord,
) -> Result<(), WorkerRpcError> {
    let unavailable = match (&offer.descriptor_set, &predecessor.state) {
        (
            CustodyDescriptorSet::MacosProcessCapability { .. },
            CustodyDurableState::PreparedGuardian { .. },
        ) => !prepared::accepts(offer, predecessor),
        (
            CustodyDescriptorSet::MacosProcessCapability { .. },
            CustodyDurableState::Retired { .. },
        ) => true,
        _ => false,
    };
    identity(!unavailable, ACTIVE_PREDECESSOR_DETAIL)
}

fn validate_released_offer(
    request: &CustodyRequest,
    offer: &CustodyOffer,
    predecessor: &CustodyDurableRecord,
) -> Result<(), WorkerRpcError> {
    identity(
        offer.descriptor_set == CustodyDescriptorSet::ReleasedSuccessor,
        OFFER_PROFILE_DETAIL,
    )?;
    let exact_exit = predecessor
        .predecessor_lease()
        .is_some_and(|lease: CustodyDurableLease| {
            lease.holder.observation() == CustodyHolderObservation::ProvenGone
                && lease.lease_id != request.lease_id
                && lease.controller_generation < request.controller_generation
                && lease.recovery_identity_sha256 == offer.recovery_identity_sha256
        });
    identity(exact_exit, RELEASED_SUCCESSOR_EVIDENCE_DETAIL)
}

pub(super) fn validate_durable_candidate(
    host: &Host,
    record: &RuntimeRecord,
    runtime: &CustodyRuntimeIdentity,
) -> Result<CustodyDurableSnapshot, WorkerRpcError> {
    let durable = read_durable_snapshot(host, record)?;
    identity(
        durable.record().is_valid() && &durable.record().runtime == runtime,
        DURABLE_IDENTITY_DETAIL,
    )?;
    Ok(durable)
}

pub(super) fn validate_committed(
    host: &Host,
    record: &RuntimeRecord,
    request: &CustodyRequest,
    offer: &CustodyOffer,
    guardian: Option<crate::pty_custody::CustodyGuardianBinding>,
) -> Result<(), WorkerRpcError> {
    let current = host
        .read_namespace_json::<RuntimeRecord>(&host.paths.runtime_record(record.terminal_id))
        .map_err(|error| WorkerRpcError::transport(super::containment::error_detail(error)))?;
    identity(&current == record, RUNTIME_PUBLICATION_DETAIL)?;
    let durable = read_durable_snapshot(host, record)?.into_record();
    identity(
        durable.matches_committed_offer(request, offer, guardian),
        DURABLE_COMMIT_DETAIL,
    )
}

fn read_durable_snapshot(
    host: &Host,
    record: &RuntimeRecord,
) -> Result<CustodyDurableSnapshot, WorkerRpcError> {
    host.namespace
        .read_json_with_identity(&host.paths.pty_custody(record.terminal_id))
        .map(|(durable, identity)| CustodyDurableSnapshot::new(durable, identity))
        .map_err(|error| {
            let detail = super::containment::error_detail(error);
            WorkerRpcError::transport(format!("{DURABLE_READ_CONTEXT}: {detail}"))
        })
}

fn identity(valid: bool, detail: &'static str) -> Result<(), WorkerRpcError> {
    valid
        .then_some(())
        .ok_or_else(|| WorkerRpcError::identity_mismatch(detail))
}

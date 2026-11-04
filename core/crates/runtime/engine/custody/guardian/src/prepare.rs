use super::protocol::{GuardianPrepareRequest, GuardianPrepared};
use super::settlement::GuardianState;
use crate::model::RuntimeRecord;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::process::MacosSecondaryCustody;
use crate::protocol::RequestEnvelope;
use crate::pty_custody::{
    CustodyDurableRecord, CustodyGuardianBinding, CustodyHolderObservation, CustodyUnixStream,
};
use anyhow::{Context, Result, ensure};
use yolo_paths::durable_mutation::SessionDurableMutationGuard;

const REQUEST_DETAIL: &str = "detached PTY custody guardian successor request is not exact";
const PREDECESSOR_DETAIL: &str = "detached PTY custody guardian predecessor changed";
const RUNTIME_DETAIL: &str = "detached PTY custody guardian runtime changed";
const PENDING_DETAIL: &str = "detached PTY custody guardian pending successor changed";

pub(super) async fn handle(
    namespace: &NamespaceLease,
    paths: &Paths,
    custody: &MacosSecondaryCustody,
    state: &mut GuardianState,
    mut stream: CustodyUnixStream,
    opening: RequestEnvelope,
) -> Result<()> {
    let payload = GuardianPrepareRequest::decode(&opening)?;
    ensure!(payload.protocol_is_current(), REQUEST_DETAIL);
    let (guardian_id, request, offer, requested_runtime_identity, requested_predecessor) =
        payload.parts();
    ensure!(guardian_id == state.binding().guardian_id(), REQUEST_DETAIL);
    super::socket::verify_peer(&stream, request.holder())?;
    ensure!(request.runtime == *state.runtime(), REQUEST_DETAIL);
    ensure!(
        offer.matches_request(&request) && offer.profile.is_active(),
        REQUEST_DETAIL
    );
    ensure!(
        custody.recovery_identity_sha256() == offer.recovery_identity_sha256,
        REQUEST_DETAIL
    );
    let preparation = install_successor(
        namespace,
        paths,
        state,
        guardian_id,
        request,
        offer,
        requested_runtime_identity,
        requested_predecessor,
    )?;
    stream.write(&GuardianPrepared::new(preparation)).await
}

#[allow(clippy::too_many_arguments)]
fn install_successor(
    namespace: &NamespaceLease,
    paths: &Paths,
    state: &mut GuardianState,
    guardian_id: uuid::Uuid,
    request: crate::pty_custody::CustodyRequest,
    offer: crate::pty_custody::CustodyOffer,
    requested_runtime_identity: FileIdentity,
    requested_predecessor: crate::pty_custody::CustodyDurableSnapshot,
) -> Result<crate::pty_custody::CustodyGuardianPreparation> {
    let guard = SessionDurableMutationGuard::acquire_participant(
        paths,
        namespace,
        request.runtime.terminal_id,
    )?;
    guard.require_no_pending_intent()?;
    let runtime_path = paths.runtime_record(request.runtime.terminal_id);
    let (runtime, runtime_identity): (RuntimeRecord, FileIdentity) =
        guard.namespace().read_json_with_identity(&runtime_path)?;
    ensure!(
        request.runtime.matches_record(&runtime)
            && request.auth_token() == runtime.auth_token
            && requested_runtime_identity == runtime_identity,
        RUNTIME_DETAIL
    );
    let (durable, durable_identity): (CustodyDurableRecord, FileIdentity) = guard
        .namespace()
        .read_json_with_identity(&paths.pty_custody(request.runtime.terminal_id))?;
    let retained = state.pending();
    ensure!(
        requested_predecessor_matches(state, &durable, durable_identity, &requested_predecessor,),
        PREDECESSOR_DETAIL
    );
    if retained.is_none() {
        validate_predecessor(state, &durable, &runtime, request.controller_generation)?;
    }
    let prepared =
        CustodyGuardianBinding::new(guardian_id, state.binding().process(), &request, &offer)
            .context(REQUEST_DETAIL)?;
    let retained_runtime_file = state
        .runtime_replacement_required(&runtime, runtime_identity)?
        .then(|| {
            super::settlement_runtime_file::RetainedRuntimeFile::capture(
                guard.namespace(),
                paths,
                &runtime,
                runtime_identity,
            )
        })
        .transpose()?;
    let runtime_update =
        state.prepare_runtime_update(&runtime, runtime_identity, retained_runtime_file)?;
    ensure!(
        durable.runtime == crate::pty_custody::CustodyRuntimeIdentity::from_record(&runtime),
        PENDING_DETAIL
    );
    let preparation = super::preparation::install_successor_guarded(
        &guard,
        paths,
        &durable,
        durable_identity,
        runtime_identity,
        &request,
        &offer,
        prepared,
        retained,
    )?;
    let predecessor = crate::pty_custody::CustodyDurableSnapshot::new(durable, durable_identity);
    let (prepared, prepared_identity): (CustodyDurableRecord, FileIdentity) = guard
        .namespace()
        .read_json_with_identity(&paths.pty_custody(request.runtime.terminal_id))?;
    ensure!(
        prepared_identity == preparation.custody_file_identity()
            && prepared
                .prepared_guardian_candidate()
                .is_some_and(|candidate| {
                    preparation.binding().matches_lease_identity(&candidate)
                }),
        PENDING_DETAIL
    );
    guard.require_no_pending_intent()?;
    state.install_prepared_pending(
        preparation,
        prepared,
        predecessor,
        runtime,
        runtime_identity,
        runtime_update,
    );
    Ok(preparation)
}

fn requested_predecessor_matches(
    state: &GuardianState,
    current: &CustodyDurableRecord,
    current_identity: FileIdentity,
    requested: &crate::pty_custody::CustodyDurableSnapshot,
) -> bool {
    if requested.record() == current && requested.file_identity() == current_identity {
        return true;
    }
    state.pending_record() == Some(current) && state.pending_predecessor() == Some(requested)
}

fn validate_predecessor(
    state: &GuardianState,
    durable: &CustodyDurableRecord,
    runtime: &RuntimeRecord,
    successor_generation: u64,
) -> Result<()> {
    ensure!(
        durable.is_valid() && durable.runtime == *state.runtime(),
        PREDECESSOR_DETAIL
    );
    let lease = durable.armed_lease().context(PREDECESSOR_DETAIL)?;
    ensure!(
        lease.guardian == Some(state.binding())
            && lease.holder.observation() == CustodyHolderObservation::ProvenGone
            && successor_generation > lease.controller_generation
            && successor_generation > runtime.controller_generation
            && state.binding().process().matches(),
        PREDECESSOR_DETAIL
    );
    Ok(())
}

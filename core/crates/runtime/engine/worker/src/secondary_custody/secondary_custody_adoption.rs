use super::response_drain::ResponsePublicationTicket;
use super::secondary_custody_adoption_deadline::WorkerAdoptionDeadline;
use super::secondary_custody_adoption_fence::CustodyAdoptionFence;
use super::secondary_custody_export::SuccessorCustodyExportLease;
use super::secondary_custody_handshake::exchange_successor;
use super::secondary_custody_handshake_source::WorkerCustodyHandshakeSource;
use super::{WorkerSecondaryCustodySource, WorkerState, lock_recover};
use crate::local_unix::LocalUnixStream;
use crate::protocol::RequestEnvelope;
use crate::pty_custody::{CustodyDurableRecord, CustodyProfile, CustodyUnixStream};
use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;

#[path = "secondary_custody_adoption_rollback.rs"]
mod rollback;
use rollback::{fail_post_ready, rollback_post_ready};

const SOURCE_UNAVAILABLE_DETAIL: &str = "PTY secondary-custody source is unavailable";
const EXPORT_CLOSURE_DETAIL: &str = "PTY successor export was fenced before custody promotion";
const PREDECESSOR_UNAVAILABLE_DETAIL: &str = "active PTY custody predecessor is unavailable";
const ADOPTION_GATE_TIMEOUT_CONTEXT: &str = "PTY successor adoption gate timed out";
pub(crate) const HANDSHAKE_TIMEOUT_CONTEXT: &str = "PTY successor custody handshake timed out";
const RELEASE_RECOVERY_IDENTITY_DETAIL: &str = "released successor has no durable predecessor";
const PROFILE_UNAVAILABLE_DETAIL: &str = "PTY successor custody profile is unavailable";
pub(crate) async fn adopt_runtime(
    stream: LocalUnixStream,
    opening: RequestEnvelope,
    state: Arc<WorkerState>,
    publication: ResponsePublicationTicket,
) -> Result<()> {
    let timeout = crate::session_timing::WORKER_READY_TIMEOUT;
    adopt_runtime_with_timeout(stream, opening, state, publication, timeout).await
}

pub(crate) async fn adopt_runtime_with_timeout(
    stream: LocalUnixStream,
    opening: RequestEnvelope,
    state: Arc<WorkerState>,
    publication: ResponsePublicationTicket,
    timeout: Duration,
) -> Result<()> {
    let deadline = WorkerAdoptionDeadline::from_timeout(timeout);
    let stream = CustodyUnixStream::from_local(stream)?;
    let request = CustodyUnixStream::decode_opening(&opening)?;
    let adoption = deadline
        .wait(
            state.secondary_custody_adoption_gate.lock(),
            ADOPTION_GATE_TIMEOUT_CONTEXT,
        )
        .await?;
    let permit = deadline.admit_system_mutation(&state).await?;
    state.reconcile_committed_custody(&permit)?;
    let source = lock_recover(&state.secondary_custody).clone();
    let released = source.is_none() && state.pty_alias_custody.released_successor_adoptable();
    let profile = state
        .adoption_profile(source.is_some(), released)
        .context(PROFILE_UNAVAILABLE_DETAIL)?;
    if profile == CustodyProfile::ActiveRunning {
        state
            .ensure_process_running()
            .map_err(|error| anyhow::anyhow!(error.message))?;
    }
    let predecessor = if released {
        None
    } else {
        Some(
            state
                .secondary_custody_authority
                .predecessor_identity()
                .context(PREDECESSOR_UNAVAILABLE_DETAIL)?,
        )
    };
    if source.is_none() && !released {
        return Err(anyhow::anyhow!(SOURCE_UNAVAILABLE_DETAIL));
    }
    let mut export = source
        .as_ref()
        .map(WorkerSecondaryCustodySource::lease_successor_export)
        .transpose()?;
    let runtime = lock_recover(&state.record).clone();
    drop(permit);

    let (durable, durable_identity): (CustodyDurableRecord, _) = state
        .namespace
        .read_json_with_identity(&state.paths.pty_custody(runtime.terminal_id))?;
    super::secondary_custody_predecessor::validate_durable(predecessor, &durable)?;
    let handshake_source = match export.as_mut() {
        Some(export) => WorkerCustodyHandshakeSource::Successor { export, profile },
        None => WorkerCustodyHandshakeSource::Released(
            durable
                .predecessor_lease()
                .context(RELEASE_RECOVERY_IDENTITY_DETAIL)?
                .recovery_identity_sha256,
        ),
    };
    let mut adoption_fence = CustodyAdoptionFence::unarmed(Arc::clone(&state));
    let exchange = exchange_successor(
        stream,
        request,
        &state.namespace,
        &state.paths,
        &runtime,
        &durable,
        durable_identity,
        handshake_source,
        &mut adoption_fence,
        publication,
    );
    let lease = deadline.wait(exchange, HANDSHAKE_TIMEOUT_CONTEXT).await??;

    let (candidate, candidate_identity) = match state
        .namespace
        .read_json_with_identity(&state.paths.pty_custody(runtime.terminal_id))
    {
        Ok(candidate) => candidate,
        Err(error) => {
            return rollback_post_ready(&state, lease, &mut export, error).await;
        }
    };
    #[cfg(test)]
    state
        .control_test_hooks
        .custody_before_promotion
        .pause_if_armed()
        .await;
    let mutation = deadline.admit_system_mutation(&state);
    let permit = match export.as_ref() {
        Some(export) => {
            tokio::select! {
                permit = mutation => Some(permit),
                () = export.wait_for_closure_request() => None,
            }
        }
        None => Some(mutation.await),
    };
    let permit = match permit {
        Some(Ok(permit)) => permit,
        Some(Err(error)) => return rollback_post_ready(&state, lease, &mut export, error).await,
        None => {
            let error = anyhow::anyhow!(EXPORT_CLOSURE_DETAIL);
            return rollback_post_ready(&state, lease, &mut export, error).await;
        }
    };
    if export
        .as_ref()
        .is_some_and(SuccessorCustodyExportLease::closure_requested)
    {
        drop(permit);
        let error = anyhow::anyhow!(EXPORT_CLOSURE_DETAIL);
        return rollback_post_ready(&state, lease, &mut export, error).await;
    }
    let promotion = match predecessor {
        Some(predecessor) => state.promote_ready_custody(
            &permit,
            &lease,
            &runtime,
            predecessor,
            &candidate,
            candidate_identity,
        ),
        None => state.promote_released_ready_custody(
            &permit,
            &lease,
            &runtime,
            &candidate,
            candidate_identity,
        ),
    };
    if let Err(error) = promotion {
        drop(permit);
        return rollback_post_ready(&state, lease, &mut export, anyhow::anyhow!(error.message))
            .await;
    }
    let identity = lease.identity();
    if predecessor.is_some() {
        adoption_fence.advance_to_successor(identity);
    }
    drop(source);
    drop(permit);
    let lease = match super::secondary_custody_activation::activate_successor_before(
        lease, deadline, &state,
    )
    .await
    {
        Ok(lease) => lease,
        Err(error) => {
            drop(export.take());
            return fail_post_ready(&state, error).await;
        }
    };
    if let Some(mut export) = export.take() {
        export.resolve();
    }
    adoption_fence.disarm();
    debug_assert_eq!(identity, lease.identity());
    drop(adoption);
    lease.hold(state).await
}

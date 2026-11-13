use super::{
    Admission, CustodyAdoptionFence, InitialCustodyHandshakeBoundary, READY_DETAIL,
    WorkerCustodyHandshakeSource, WorkerCustodyLease, WorkerNamespace, guardian_validation,
    transition,
};
use crate::model::RuntimeRecord;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::{
    CustodyDurableRecord, CustodyHostAck, CustodyOffer, CustodyRequest, CustodyUnixStream,
    CustodyWorkerCommit,
};
use anyhow::{Result, ensure};

pub(crate) async fn complete(
    mut stream: CustodyUnixStream,
    namespace: &WorkerNamespace,
    paths: &Paths,
    runtime: &RuntimeRecord,
    durable: &CustodyDurableRecord,
    durable_identity: FileIdentity,
    request: &CustodyRequest,
    offer: &CustodyOffer,
    prepared_ack: CustodyHostAck,
    source: WorkerCustodyHandshakeSource<'_>,
    admission: Admission,
    adoption_fence: Option<&mut CustodyAdoptionFence>,
) -> Result<WorkerCustodyLease> {
    let guardian_preparation = prepared_ack.prepared_guardian();
    let validated = guardian_validation::validate(
        namespace,
        paths,
        runtime,
        durable,
        durable_identity,
        request,
        offer,
        guardian_preparation,
    )?;
    let (expected, armed) = transition::build(
        admission,
        durable,
        request,
        offer,
        validated.prepared.as_ref().map(|prepared| &prepared.record),
    )?;
    source.inject_initial_fault(InitialCustodyHandshakeBoundary::DurablePersist)?;
    let committed_identity =
        super::super::secondary_custody_persistence::persist_transition_with_boundary(
            namespace,
            paths,
            runtime,
            validated.runtime_identity,
            &expected,
            validated
                .prepared
                .as_ref()
                .map(|prepared| prepared.identity)
                .unwrap_or(durable_identity),
            &armed,
            || source.inject_initial_fault(InitialCustodyHandshakeBoundary::DurableInstalled),
        )?;
    if let Some(fence) = adoption_fence {
        fence.arm_committed(runtime, armed, committed_identity);
    }
    source.inject_initial_fault(InitialCustodyHandshakeBoundary::CommitWrite)?;
    stream.write(&CustodyWorkerCommit::new(offer)).await?;
    let ready: CustodyHostAck = stream.read().await?;
    ensure!(
        ready.matches_ready(offer, request.controller_generation),
        READY_DETAIL
    );
    WorkerCustodyLease::new(
        stream,
        offer.clone(),
        request.holder(),
        request.controller_generation,
        guardian_preparation.map(|preparation| preparation.binding()),
    )
}

use super::response_drain::ResponsePublicationTicket;
use super::secondary_custody_adoption_fence::CustodyAdoptionFence;
use super::secondary_custody_handshake_source::WorkerCustodyHandshakeSource;
use super::secondary_custody_source::InitialCustodyHandshakeBoundary;
use super::{WorkerCustodyLease, WorkerNamespace, WorkerSecondaryCustodySource};
use crate::model::RuntimeRecord;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::protocol::RequestEnvelope;
use crate::pty_custody::{
    CustodyDurableRecord, CustodyHostAck, CustodyOffer, CustodyProfile, CustodyRequest,
    CustodyUnixStream,
};
use anyhow::{Context, Result, ensure};
use std::sync::Arc;
use tokio::net::UnixListener;

#[path = "secondary_custody_handshake_commit.rs"]
mod commit;
#[path = "secondary_custody_handshake_generation.rs"]
mod generation;
#[path = "secondary_custody_guardian_validation.rs"]
mod guardian_validation;
#[path = "secondary_custody_handshake_initial.rs"]
mod initial;
#[path = "secondary_custody_handshake_transition.rs"]
mod transition;

pub(crate) use initial::establish as establish_initial;

const HOLDER_IDENTITY_DETAIL: &str = "PTY custody holder identity does not match its peer";
const REQUEST_DETAIL: &str = "PTY custody request does not match the pending runtime";
const PREPARED_DETAIL: &str = "PTY custody Prepared acknowledgement does not match its offer";
const READY_DETAIL: &str = "PTY custody Ready acknowledgement does not match its offer";
#[derive(Clone, Copy)]
enum Admission {
    Initial,
    Successor,
}

pub(crate) async fn accept_valid(
    listener: &UnixListener,
    namespace: &WorkerNamespace,
    paths: &Paths,
    runtime: &RuntimeRecord,
    durable: &CustodyDurableRecord,
    durable_identity: FileIdentity,
    source: Arc<WorkerSecondaryCustodySource>,
) -> Result<WorkerCustodyLease> {
    loop {
        let (accepted, _) = listener.accept().await?;
        let Ok(mut stream) = CustodyUnixStream::from_accepted(accepted) else {
            continue;
        };
        let Ok(opening) = stream.read::<RequestEnvelope>().await else {
            continue;
        };
        let Ok(request) = CustodyUnixStream::decode_opening(&opening) else {
            continue;
        };
        match exchange(
            stream,
            request,
            namespace,
            paths,
            runtime,
            durable,
            durable_identity,
            WorkerCustodyHandshakeSource::Initial {
                source: &source,
                profile: CustodyProfile::ActiveRunning,
            },
            Admission::Initial,
            None,
            None,
        )
        .await
        {
            Ok(lease) => return Ok(lease),
            Err(error) if source.transfer_possible() => return Err(error),
            Err(_) => {}
        }
    }
}

pub(crate) async fn exchange_successor(
    stream: CustodyUnixStream,
    request: CustodyRequest,
    namespace: &WorkerNamespace,
    paths: &Paths,
    runtime: &RuntimeRecord,
    durable: &CustodyDurableRecord,
    durable_identity: FileIdentity,
    source: WorkerCustodyHandshakeSource<'_>,
    adoption_fence: &mut CustodyAdoptionFence,
    publication: ResponsePublicationTicket,
) -> Result<WorkerCustodyLease> {
    exchange(
        stream,
        request,
        namespace,
        paths,
        runtime,
        durable,
        durable_identity,
        source,
        Admission::Successor,
        Some(adoption_fence),
        Some(publication),
    )
    .await
}

async fn exchange(
    mut stream: CustodyUnixStream,
    request: CustodyRequest,
    namespace: &WorkerNamespace,
    paths: &Paths,
    runtime: &RuntimeRecord,
    durable: &CustodyDurableRecord,
    durable_identity: FileIdentity,
    mut source: WorkerCustodyHandshakeSource<'_>,
    admission: Admission,
    adoption_fence: Option<&mut CustodyAdoptionFence>,
    mut publication: Option<ResponsePublicationTicket>,
) -> Result<WorkerCustodyLease> {
    ensure!(
        request.is_well_formed()
            && request.uses_current_protocol()
            && request.runtime.matches_record(runtime)
            && request.auth_token() == runtime.auth_token
            && generation::matches(admission, &request, runtime, durable),
        REQUEST_DETAIL
    );
    ensure!(
        stream.peer_pid()? == request.holder().pid && request.holder().matches(),
        HOLDER_IDENTITY_DETAIL
    );
    let offer = CustodyOffer::new_with_profile(
        &request,
        source.descriptor_set(),
        source.profile(),
        source.recovery_identity_sha256(),
    )?;
    #[cfg(test)]
    super::response_drain::pause_before_write(publication.as_ref()).await;
    stream.write(&offer).await?;
    source.send_descriptors(&stream, request.holder()).await?;
    if let Some(publication) = publication.take() {
        publication.publish();
    }
    source.inject_initial_fault(InitialCustodyHandshakeBoundary::PreparedRead)?;
    let prepared: CustodyHostAck = stream.read().await?;
    ensure!(prepared.matches_prepared(&offer), PREPARED_DETAIL);
    commit::complete(
        stream,
        namespace,
        paths,
        runtime,
        durable,
        durable_identity,
        &request,
        &offer,
        prepared,
        source,
        admission,
        adoption_fence,
    )
    .await
}

#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/secondary_custody/secondary_custody_guardian_replacement_process_tests.rs"]
mod guardian_replacement_process_tests;

use super::*;
use crate::paths::atomic_write_json;
use crate::process::MacosSecondaryCustodyExport;
use crate::pty_custody::{
    CustodyHostAck, CustodyRequest, CustodyWorkerActivated, CustodyWorkerCommit,
    CustodyWorkerReadyDisposition,
};

const FORGED_GENERATION_OFFSET: u64 = 1;

type Handshake = (CustodyUnixStream, CustodyOffer, CustodyRequest);

pub(super) async fn authenticate(
    listener: &UnixListener,
    record: &RuntimeRecord,
) -> crate::local_unix::LocalUnixStream {
    let (stream, _) = listener.accept().await.unwrap();
    let mut stream = crate::local_unix::LocalUnixStream::from(stream);
    let request: crate::protocol::RequestEnvelope =
        crate::protocol::read_frame(&mut stream).await.unwrap();
    assert_eq!(request.method, crate::worker::WORKER_AUTH_METHOD);
    let hello = crate::model::WorkerHello {
        runtime: record.clone(),
        capabilities: Vec::new(),
    };
    let response = crate::protocol::ResponseEnvelope::success(&request, &hello).unwrap();
    crate::protocol::write_frame(&mut stream, &response)
        .await
        .unwrap();
    stream
}

pub(super) async fn complete_handshake(
    listener: &UnixListener,
    export: MacosSecondaryCustodyExport,
    pending: CustodyDurableRecord,
    custody_path: &std::path::Path,
    record: &RuntimeRecord,
) -> Handshake {
    let (mut stream, offer, request) =
        through_ready(listener, export, pending, custody_path, record).await;
    let activation = CustodyWorkerActivated::new(&offer, request.controller_generation).unwrap();
    stream
        .write(&CustodyWorkerReadyDisposition::activated(activation))
        .await
        .unwrap();
    (stream, offer, request)
}

pub(super) async fn close_without_activation(
    listener: &UnixListener,
    export: MacosSecondaryCustodyExport,
    pending: CustodyDurableRecord,
    custody_path: &std::path::Path,
    record: &RuntimeRecord,
) {
    drop(through_ready(listener, export, pending, custody_path, record).await);
}

pub(super) async fn send_forged_activation(
    listener: &UnixListener,
    export: MacosSecondaryCustodyExport,
    pending: CustodyDurableRecord,
    custody_path: &std::path::Path,
    record: &RuntimeRecord,
) {
    let (mut stream, offer, request) =
        through_ready(listener, export, pending, custody_path, record).await;
    let forged_generation = request
        .controller_generation
        .checked_add(FORGED_GENERATION_OFFSET)
        .unwrap_or_else(|| {
            request
                .controller_generation
                .saturating_sub(FORGED_GENERATION_OFFSET)
        });
    assert_ne!(forged_generation, request.controller_generation);
    let activation = CustodyWorkerActivated::new(&offer, forged_generation).unwrap();
    stream
        .write(&CustodyWorkerReadyDisposition::activated(activation))
        .await
        .unwrap();
}

pub(super) async fn hold_after_prepared(
    listener: &UnixListener,
    export: MacosSecondaryCustodyExport,
    record: &RuntimeRecord,
    prepared: tokio::sync::oneshot::Sender<()>,
    release: tokio::sync::oneshot::Receiver<()>,
) {
    let handshake = through_prepared(listener, export, record).await;
    prepared.send(()).unwrap();
    let _ = release.await;
    drop(handshake);
}

pub(super) async fn through_ready(
    listener: &UnixListener,
    export: MacosSecondaryCustodyExport,
    pending: CustodyDurableRecord,
    custody_path: &std::path::Path,
    record: &RuntimeRecord,
) -> Handshake {
    let (mut stream, offer, request) = through_prepared(listener, export, record).await;
    let guarded =
        crate::paths::read_json::<crate::pty_custody::CustodyDurableRecord>(custody_path).unwrap();
    let armed = match guarded
        .prepared_guardian_candidate()
        .and_then(|candidate| candidate.guardian)
    {
        Some(binding) => guarded
            .arm_prepared_guardian(&request, &offer, binding)
            .unwrap(),
        None => crate::host::test_arm_custody(&pending, &request, &offer).unwrap(),
    };
    atomic_write_json(custody_path, &armed).unwrap();
    stream
        .write(&CustodyWorkerCommit::new(&offer))
        .await
        .unwrap();
    let ready: CustodyHostAck = stream.read().await.unwrap();
    assert!(ready.matches_ready(&offer, request.controller_generation));
    (stream, offer, request)
}

pub(super) async fn through_prepared(
    listener: &UnixListener,
    export: MacosSecondaryCustodyExport,
    record: &RuntimeRecord,
) -> Handshake {
    let (accepted, _) = listener.accept().await.unwrap();
    let mut stream = CustodyUnixStream::from_accepted(accepted).unwrap();
    let opening: crate::protocol::RequestEnvelope = stream.read().await.unwrap();
    let request = CustodyUnixStream::decode_opening(&opening).unwrap();
    assert!(request.runtime.matches_record(record));
    assert_eq!(request.holder(), CustodyHolderIdentity::capture().unwrap());
    assert_eq!(request.controller_generation, record.controller_generation);
    let offer = crate::host::test_custody_offer(
        &request,
        export.descriptor_set(),
        export.recovery_identity_sha256(),
    );
    stream.write(&offer).await.unwrap();
    stream.send_rights(&export.as_descriptors()).await.unwrap();
    let prepared: CustodyHostAck = stream.read().await.unwrap();
    assert!(prepared.matches_prepared(&offer));
    (stream, offer, request)
}

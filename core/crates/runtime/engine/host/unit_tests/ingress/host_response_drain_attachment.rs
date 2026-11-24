use crate::host::tests::*;
const RESPONSE_DRAIN_ATTACHMENT_SOCKET: &str = "response-drain-attachment.sock";

#[tokio::test]
async fn host_response_drain_established_attachment_stream_holds_no_ticket() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let socket = directory.path().join(RESPONSE_DRAIN_ATTACHMENT_SOCKET);
    let listener = UnixListener::bind(&socket).unwrap();
    let (terminal_id, record) = attachment_record(&host, socket);
    let (release_worker, worker_release) = tokio::sync::oneshot::channel();
    let worker = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = UnixStream::from(stream);
        let mut stream = accept_worker_auth(stream, &record).await;
        let request: RequestEnvelope = stream.request().await.unwrap();
        let response =
            ResponseEnvelope::success(&request, &empty_attachment(record.runtime_epoch)).unwrap();
        stream.respond(&response).await.unwrap();
        worker_release.await.unwrap();
    });
    let attach = request(
        method::ATTACH_SESSION,
        &AttachRequest {
            terminal_id,
            after_seq: u64::default(),
        },
    );
    use yolo_capnp_rpc::Service;
    let mut reply = host
        .call(
            yolo_capnp_rpc::Method::Call,
            yolo_capnp_codec::to_value(&attach).unwrap(),
        )
        .await
        .unwrap();
    let initial: ResponseEnvelope = yolo_capnp_codec::from_value(reply.value).unwrap();
    assert!(initial.ok);
    assert!(
        !host
            .request_drain
            .wait_for_response_drain(Duration::ZERO)
            .await
    );
    reply.on_observed.take().unwrap().confirm().await.unwrap();
    host.request_drain
        .fail_stop(HOST_RESPONSE_DRAIN_TEST_DETAIL);

    assert!(
        host.request_drain
            .wait_for_response_drain(Duration::ZERO)
            .await
    );
    let mut events = reply.events.take().unwrap();
    let pending = events.next();
    tokio::pin!(pending);
    assert!(futures::poll!(pending.as_mut()).is_pending());
    release_worker.send(()).unwrap();
    assert!(pending.await.is_err());
    worker.await.unwrap();
}

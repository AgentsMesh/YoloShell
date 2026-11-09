use crate::host::tests::*;
use super::*;

const DELIVERY_FAILURE_SOCKET: &str = "delivery-failure.sock";
const DELIVERY_PAYLOAD_DIVISOR: usize = 16;
const DELIVERY_PAYLOAD_BYTE: u8 = b'x';
const DELIVERY_SETTLE_DELAY: Duration = Duration::from_millis(50);

#[tokio::test]
async fn cancelling_an_observer_with_queued_binary_output_closes_only_its_subscription() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join(DELIVERY_FAILURE_SOCKET);
    let listener = UnixListener::bind(&socket).unwrap();
    let (terminal_id, record) = attachment_record(&host, socket);
    let event_epoch = record.runtime_epoch;
    let (emit_tx, emit_rx) = tokio::sync::oneshot::channel();
    let (emitted_tx, emitted_rx) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = UnixStream::from(stream);
        let mut stream = accept_worker_auth(stream, &record).await;
        let request: RequestEnvelope = stream.request().await.unwrap();
        let attached = empty_attachment(record.runtime_epoch);
        stream
            .respond(&ResponseEnvelope::success(&request, &attached).unwrap())
            .await
            .unwrap();
        emit_rx.await.unwrap();
        let payload = yolo_capnp_value::Value::Data(vec![
            DELIVERY_PAYLOAD_BYTE;
            crate::protocol::MAX_FRAME_BYTES
                / DELIVERY_PAYLOAD_DIVISOR
        ]);
        let event = crate::protocol::EventEnvelope::new(
            crate::protocol::event::SESSION_UPDATED,
            Some(terminal_id.to_string()),
            Some(event_epoch.to_string()),
            &payload,
        )
        .unwrap();
        stream.event(&event).await.unwrap();
        emitted_tx.send(()).unwrap();
        stream.request().await
    });
    let attach = request(
        method::ATTACH_SESSION,
        &AttachRequest {
            terminal_id,
            after_seq: u64::default(),
        },
    );
    let (initial, events, publication) = host.rpc_attachment(&attach).await.unwrap();
    assert!(initial.ok);
    drop(publication);
    emit_tx.send(()).unwrap();
    emitted_rx.await.unwrap();
    drop(events);
    assert!(matches!(
        server.await.unwrap(),
        Err(ProtocolError::EndOfStream)
    ));
}

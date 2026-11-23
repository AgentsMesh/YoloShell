use super::*;
use yolo_capnp_rpc::{Connection, Method, Protocol, RemoteReply};

const FIRST_ATTACH_REQUEST: &str = "attach-1";
const NESTED_VERSION_REQUEST: &str = "nested-version";
const NESTED_INPUT_REQUEST: &str = "nested-input";
const CONTROL_HOLDER: &str = "separate-control";
const REJECTED_INPUT: &[u8] = b"attachment-control-must-not-run\n";
const INPUT_COMMAND: &[u8] = b"printf 'CAPNP_RPC_%s\\n' OUTPUT\r";
const OUTPUT_MARKER: &[u8] = b"CAPNP_RPC_OUTPUT";
const INCOMPATIBLE_VERSION: u16 = 99;
const REJECTED_GENERATION: u64 = 1;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn event_capabilities_preserve_writer_fencing_and_allow_independent_control() {
    let harness = Harness::start().await;
    let session = harness.create_cat("multiplex").await;
    let id = session.manifest.terminal_id;
    let target = runtime_ref_from_view(&session);
    let stream = tokio::net::UnixStream::connect(harness.paths().host_socket().unwrap())
        .await
        .unwrap();
    let connection = Connection::open(stream, Protocol::Endpoint, deadline())
        .await
        .unwrap();
    let attach = RequestEnvelope::new(
        FIRST_ATTACH_REQUEST,
        method::ATTACH_SESSION,
        to_value(&AttachRequest {
            terminal_id: id,
            after_seq: 0,
        })
        .unwrap(),
    );
    let attached = call(&connection, &attach).await;
    assert!(from_value::<ResponseEnvelope>(attached.value).unwrap().ok);
    let mut events = attached.events.unwrap();

    let mut incompatible = RequestEnvelope::new(
        NESTED_VERSION_REQUEST,
        method::HOST_STATUS,
        yolo_capnp_value::Value::Null,
    );
    incompatible.version = INCOMPATIBLE_VERSION;
    let response: ResponseEnvelope =
        from_value(call(&connection, &incompatible).await.value).unwrap();
    assert_eq!(
        response.error.unwrap().code,
        error_code::PROTOCOL_INCOMPATIBLE
    );

    let input = RequestEnvelope::new(
        NESTED_INPUT_REQUEST,
        method::TERMINAL_INPUT,
        to_value(&InputRequest {
            terminal_id: id,
            runtime_epoch: target.runtime_epoch,
            generation: REJECTED_GENERATION,
            writer_capability: WriterCapability::generate(),
            input_id: Uuid::new_v4(),
            data: REJECTED_INPUT.to_vec(),
        })
        .unwrap(),
    );
    let response: ResponseEnvelope = from_value(call(&connection, &input).await.value).unwrap();
    assert!(!response.ok);
    assert!(
        harness
            .client()
            .get_writer_lease(target)
            .await
            .unwrap()
            .is_none()
    );

    // An idle subscription does not occupy the endpoint's control capability.
    let client = harness.client();
    let lease = client
        .acquire_writer(target, CONTROL_HOLDER, None)
        .await
        .unwrap();
    let waiting = async {
        let mut output = Vec::new();
        loop {
            let event: EventEnvelope = from_value(events.next(deadline()).await.unwrap()).unwrap();
            if event.event == event::TERMINAL_OUTPUT {
                let chunk: OutputChunk = from_value(event.payload).unwrap();
                output.extend(chunk.data);
                assert!(
                    !output
                        .windows(REJECTED_INPUT.len())
                        .any(|bytes| bytes == REJECTED_INPUT)
                );
                if output
                    .windows(OUTPUT_MARKER.len())
                    .any(|bytes| bytes == OUTPUT_MARKER)
                {
                    return;
                }
            }
        }
    };
    let writing = client.input(
        target,
        lease.generation,
        &lease.writer_capability,
        INPUT_COMMAND.to_vec(),
    );
    let ((), written) = tokio::join!(waiting, writing);
    written.unwrap();
    drop(events);
    connection.close();
    client.release_writer(&lease, target).await.unwrap();
    let reattached = client.attach(id, 0).await.unwrap();
    assert_eq!(reattached.runtime_epoch(), target.runtime_epoch);
    assert_eq!(
        client
            .get_session(id)
            .await
            .unwrap()
            .runtime
            .unwrap()
            .status,
        TerminalRuntimeStatus::Running
    );
}

fn deadline() -> tokio::time::Instant {
    tokio::time::Instant::now() + PROCESS_EXIT_TIMEOUT
}
async fn call(connection: &Connection, request: &RequestEnvelope) -> RemoteReply {
    connection
        .request(Method::Call, to_value(request).unwrap(), deadline())
        .await
        .unwrap()
}

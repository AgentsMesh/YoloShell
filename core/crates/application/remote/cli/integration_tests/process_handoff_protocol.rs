use super::fixture;
use yolo_runtime::host::HelloResult;
use yolo_runtime::model::{HostStatus, PROTOCOL_VERSION};
use yolo_runtime::protocol::{
    HOST_SERVER_NAME, RequestEnvelope, ResponseEnvelope, capability, method, to_value,
};

const PROBE_HELLO_REQUEST_ID: &str = "probe-hello";
const PROBE_STATUS_REQUEST_ID: &str = "probe-status";
const FORWARDED_REQUEST_ID: &str = "forwarded";

pub(super) fn runtime_response(request: &RequestEnvelope) -> ResponseEnvelope {
    match request.method.as_str() {
        method::HELLO => ResponseEnvelope::success(
            request,
            &HelloResult {
                server_name: HOST_SERVER_NAME.into(),
                protocol_version: PROTOCOL_VERSION,
                capabilities: runtime_capabilities(),
            },
        ),
        method::HOST_STATUS => ResponseEnvelope::success(
            request,
            &HostStatus {
                protocol_version: PROTOCOL_VERSION,
                boot_id: fixture::PROCESS_BOOT_ID.into(),
                host_pid: fixture::PROCESS_HOST_PID,
                controller_generation: fixture::PROCESS_CONTROLLER_GENERATION,
                home: fixture::TEST_HOME.into(),
                capabilities: runtime_capabilities(),
            },
        ),
        _ => ResponseEnvelope::success(request, &()),
    }
    .unwrap()
}

fn runtime_capabilities() -> Vec<String> {
    [
        capability::DURABLE_PTY,
        capability::HOST_ADOPTION,
        capability::CAPNP_BINARY,
        capability::OUTPUT_JOURNAL,
        capability::SSH_STDIO_BRIDGE,
        capability::WRITER_LEASE_V2,
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

#[test]
fn fake_runtime_supports_pool_probe_before_forwarded_requests() {
    let hello_request = RequestEnvelope::new(
        PROBE_HELLO_REQUEST_ID,
        method::HELLO,
        to_value(&()).unwrap(),
    );
    let hello: HelloResult = runtime_response(&hello_request).decode_payload().unwrap();
    assert_eq!(hello.server_name, HOST_SERVER_NAME);
    assert_eq!(hello.protocol_version, PROTOCOL_VERSION);
    assert_eq!(hello.capabilities, runtime_capabilities());

    let status_request = RequestEnvelope::new(
        PROBE_STATUS_REQUEST_ID,
        method::HOST_STATUS,
        to_value(&()).unwrap(),
    );
    let status: HostStatus = runtime_response(&status_request).decode_payload().unwrap();
    assert_eq!(status.home, std::path::Path::new(fixture::TEST_HOME));
    assert_eq!(status.capabilities, runtime_capabilities());

    let forwarded = RequestEnvelope::new(
        FORWARDED_REQUEST_ID,
        method::ATTACH_SESSION,
        to_value(&()).unwrap(),
    );
    let _: () = runtime_response(&forwarded).decode_payload().unwrap();
}

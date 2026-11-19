use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn local_ipc_accepts_the_effective_user_and_rejects_other_uids() {
    let (_client, server) = UnixStream::pair().unwrap();
    verify_peer_uid(&server).unwrap();
    // SAFETY: geteuid has no preconditions and only reads process credentials.
    let uid = unsafe { libc::geteuid() };
    validate_peer_uid(uid, uid).unwrap();
    assert!(validate_peer_uid(uid.wrapping_add(1), uid).is_err());
}

#[tokio::test]
async fn connected_client_gets_protocol_errors_without_losing_the_socket() {
    const INCOMPATIBLE_PROTOCOL_VERSION_OFFSET: u16 = 1;
    const INCOMPATIBLE_PROTOCOL_VERSION: u16 =
        crate::model::PROTOCOL_VERSION.saturating_add(INCOMPATIBLE_PROTOCOL_VERSION_OFFSET);
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let (mut client, server) = UnixStream::pair().unwrap();
    let task = tokio::spawn(Arc::clone(&host).handle_client(server));
    let client = open_host_rpc(client).await;
    let mut incompatible = request(method::HOST_STATUS, &());
    incompatible.version = INCOMPATIBLE_PROTOCOL_VERSION;
    let response = host_rpc(&client, &incompatible).await;
    let incompatible = response.error.unwrap();
    assert_eq!(
        incompatible.code,
        crate::protocol::error_code::PROTOCOL_INCOMPATIBLE
    );
    assert!(
        incompatible
            .message
            .contains(&crate::model::PROTOCOL_VERSION.to_string())
    );
    let unsupported = request("not-implemented", &());
    let response = host_rpc(&client, &unsupported).await;
    assert_eq!(
        response.error.unwrap().code,
        crate::protocol::error_code::UNSUPPORTED_METHOD
    );
    let malformed_attach = RequestEnvelope::new(
        "malformed-attach",
        method::ATTACH_SESSION,
        yolo_capnp_value::Value::Null,
    );
    let response = host_rpc(&client, &malformed_attach).await;
    assert_eq!(
        response.error.unwrap().code,
        crate::protocol::error_code::INVALID_REQUEST
    );
    let attach = request(
        method::ATTACH_SESSION,
        &AttachRequest {
            terminal_id: Uuid::new_v4(),
            after_seq: 0,
        },
    );
    let response = host_rpc(&client, &attach).await;
    assert_eq!(
        response.error.unwrap().code,
        crate::protocol::error_code::WORKER_UNREACHABLE
    );
    drop(client);
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn runtime_records_reject_session_pid_and_boot_identity_mismatches() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let requested = Uuid::new_v4();
    host.paths.ensure_session(requested).unwrap();
    let mut record = RuntimeRecord {
        schema_version: RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: std::process::id(),
        worker_start_time: process::process_start_time(std::process::id()).unwrap_or(0),
        child_pid: std::process::id(),
        child_start_time: process::process_start_time(std::process::id()).unwrap_or(0),
        boot_id: host.boot_id.clone(),
        socket_path: host.paths.worker_socket(requested).unwrap(),
        auth_token: "token".into(),
        controller_generation: host.controller_generation,
        started_at_ms: now_ms(),
        command: "/bin/cat".into(),
        args: Vec::new(),
        cwd: directory.path().to_path_buf(),
        cols: 80,
        rows: 24,
        bootstrap_delivery: crate::model::BootstrapDelivery::NotRequired,
    };
    atomic_write_json(&host.paths.runtime_record(requested), &record).unwrap();
    assert_eq!(
        host.live_runtime_record(requested).await.unwrap_err().code,
        crate::protocol::error_code::IDENTITY_MISMATCH
    );
    record.terminal_id = requested;
    record.worker_pid = u32::MAX;
    atomic_write_json(&host.paths.runtime_record(requested), &record).unwrap();
    assert_eq!(
        host.live_runtime_record(requested).await.unwrap_err().code,
        crate::protocol::error_code::WORKER_UNREACHABLE
    );
    record.worker_pid = std::process::id();
    record.worker_start_time = record.worker_start_time.wrapping_add(1);
    atomic_write_json(&host.paths.runtime_record(requested), &record).unwrap();
    assert_eq!(
        host.live_runtime_record(requested).await.unwrap_err().code,
        crate::protocol::error_code::IDENTITY_MISMATCH
    );
    record.worker_start_time = process::process_start_time(std::process::id()).unwrap_or(0);
    atomic_write_json(&host.paths.runtime_record(requested), &record).unwrap();
    let error = host
        .rpc_attachment(&request(
            method::ATTACH_SESSION,
            &AttachRequest {
                terminal_id: requested,
                after_seq: 0,
            },
        ))
        .await
        .err()
        .unwrap();
    assert_eq!(error.code, crate::protocol::error_code::WORKER_UNREACHABLE);
}

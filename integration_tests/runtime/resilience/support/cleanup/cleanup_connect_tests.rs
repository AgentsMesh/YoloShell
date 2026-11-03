use super::*;
use std::io;
use std::os::unix::net::{UnixListener, UnixStream};

const FIXTURE_SOCKET_NAME: &str = "worker.sock";
const FIXTURE_AUTH_TOKEN: &str = "cleanup-auth-token";
const FIXTURE_BOOT_ID: &str = "cleanup-boot";
const FIXTURE_COMMAND: &str = "/bin/sh";
const FIXTURE_CONTROLLER_GENERATION: u64 = 7;
const FIXTURE_MISMATCH_INCREMENT: u64 = 1;
const FIXTURE_STARTED_AT_MS: i64 = 0;
const FIXTURE_COLUMNS: u16 = 80;
const FIXTURE_ROWS: u16 = 24;
const FIXTURE_SERVER_ACCEPT_TIMEOUT: Duration = Duration::from_secs(3);
const FIXTURE_SERVER_ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(1);
const FIXTURE_SERVER_ACCEPT_TIMEOUT_DETAIL: &str = "fixture cleanup server accept timed out";
const FIXTURE_SERVER_ACCEPT_ERROR_DETAIL: &str = "fixture cleanup server accept failed";

#[path = "cleanup_connect_pending_tests.rs"]
mod pending_tests;

#[test]
fn worker_cleanup_authenticates_and_sends_termination_over_bounded_connect() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(FIXTURE_SOCKET_NAME);
    let listener = UnixListener::bind(&path).unwrap();
    listener.set_nonblocking(true).unwrap();
    let record = fixture_record(path);
    let expected = record.clone();
    let worker = std::thread::spawn(move || serve_cleanup(listener, &expected));

    let worker_identity = wait_for_thread_identity();
    let record = record_for_worker(record, worker_identity);

    assert!(request_worker_termination(&record));
    worker.join().unwrap();
}

#[test]
fn mismatched_authenticated_runtime_never_receives_termination() {
    for mismatch in RuntimeMismatch::ALL {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(FIXTURE_SOCKET_NAME);
        let listener = UnixListener::bind(&path).unwrap();
        listener.set_nonblocking(true).unwrap();
        let record = fixture_record(path);
        let authenticated = mismatch.apply(&record);
        let worker = std::thread::spawn(move || serve_mismatch(listener, authenticated));

        let worker_identity = wait_for_thread_identity();
        let record = record_for_worker(record, worker_identity);

        assert!(!request_worker_termination(&record));
        assert!(!worker.join().unwrap());
    }
}

fn wait_for_thread_identity() -> ProcessIdentity {
    ProcessIdentity::capture(std::process::id()).unwrap()
}

fn record_for_worker(mut record: RuntimeRecord, identity: ProcessIdentity) -> RuntimeRecord {
    record.worker_pid = identity.pid;
    record.worker_start_time = identity.start_time;
    record
}

fn serve_cleanup(listener: UnixListener, expected: &RuntimeRecord) {
    let mut stream = accept_cleanup(&listener);
    stream.set_nonblocking(false).unwrap();
    stream.set_read_timeout(Some(CLEANUP_RPC_TIMEOUT)).unwrap();
    stream.set_write_timeout(Some(CLEANUP_RPC_TIMEOUT)).unwrap();
    let auth: RequestEnvelope = read_frame_blocking(&mut stream).unwrap();
    assert_eq!(auth.request_id, CLEANUP_AUTH_REQUEST_ID);
    assert_eq!(auth.method, WORKER_AUTH_METHOD);
    let payload: WorkerAuth = auth.decode_payload().unwrap();
    assert_eq!(payload.token, expected.auth_token);
    assert_eq!(
        payload.controller_generation,
        expected.controller_generation
    );
    write_auth_response(&mut stream, &auth, expected);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        stream.set_nonblocking(true).unwrap();
        let mut peer =
            yolo_runtime_rpc_testing::Peer::new(tokio::net::UnixStream::from_std(stream).unwrap());
        let terminate = peer.request().await.unwrap();
        assert_eq!(terminate.request_id, CLEANUP_TERMINATE_REQUEST_ID);
        assert_eq!(terminate.method, method::TERMINATE_RUNTIME_EXACT);
        let payload: FencedTerminateRequest = terminate.decode_payload().unwrap();
        assert_eq!(payload.terminal_id, expected.terminal_id);
        assert_eq!(payload.runtime_epoch, expected.runtime_epoch);
        assert_eq!(payload.expected_status, None);
        assert!(!payload.force);
        peer.respond(&ResponseEnvelope::empty(&terminate))
            .await
            .unwrap();
        assert!(matches!(
            peer.request().await,
            Err(ProtocolError::EndOfStream)
        ));
    });
}

fn serve_mismatch(listener: UnixListener, authenticated: RuntimeRecord) -> bool {
    let mut stream = accept_cleanup(&listener);
    stream.set_nonblocking(false).unwrap();
    stream.set_read_timeout(Some(CLEANUP_RPC_TIMEOUT)).unwrap();
    stream.set_write_timeout(Some(CLEANUP_RPC_TIMEOUT)).unwrap();
    let auth: RequestEnvelope = read_frame_blocking(&mut stream).unwrap();
    write_auth_response(&mut stream, &auth, &authenticated);
    read_frame_blocking::<_, RequestEnvelope>(&mut stream).is_ok()
}

fn write_auth_response(stream: &mut UnixStream, auth: &RequestEnvelope, record: &RuntimeRecord) {
    let hello = yolo_runtime::model::WorkerHello {
        runtime: record.clone(),
        capabilities: Vec::new(),
    };
    let response = ResponseEnvelope::success(auth, &hello).unwrap();
    write_frame_blocking(stream, &response).unwrap();
}

fn accept_cleanup(listener: &UnixListener) -> UnixStream {
    let deadline = Instant::now() + FIXTURE_SERVER_ACCEPT_TIMEOUT;
    loop {
        match listener.accept() {
            Ok((stream, _)) => return stream,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                assert!(
                    Instant::now() < deadline,
                    "{FIXTURE_SERVER_ACCEPT_TIMEOUT_DETAIL}"
                );
                std::thread::sleep(FIXTURE_SERVER_ACCEPT_POLL_INTERVAL);
            }
            Err(error) => panic!("{FIXTURE_SERVER_ACCEPT_ERROR_DETAIL}: {error}"),
        }
    }
}

fn fixture_record(socket_path: PathBuf) -> RuntimeRecord {
    let process_id = std::process::id();
    let process_start_time = ProcessIdentity::capture(process_id).unwrap().start_time;
    RuntimeRecord {
        schema_version: yolo_runtime::constants::RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: process_id,
        worker_start_time: process_start_time,
        child_pid: process_id,
        child_start_time: process_start_time,
        boot_id: FIXTURE_BOOT_ID.into(),
        socket_path,
        auth_token: FIXTURE_AUTH_TOKEN.into(),
        controller_generation: FIXTURE_CONTROLLER_GENERATION,
        started_at_ms: FIXTURE_STARTED_AT_MS,
        command: FIXTURE_COMMAND.into(),
        args: Vec::new(),
        cwd: std::env::temp_dir(),
        cols: FIXTURE_COLUMNS,
        rows: FIXTURE_ROWS,
        bootstrap_delivery: yolo_runtime::model::BootstrapDelivery::NotRequired,
    }
}

#[derive(Clone, Copy)]
enum RuntimeMismatch {
    ControllerGeneration,
    WorkerStartTime,
    ChildStartTime,
}

impl RuntimeMismatch {
    const ALL: [Self; 3] = [
        Self::ControllerGeneration,
        Self::WorkerStartTime,
        Self::ChildStartTime,
    ];

    fn apply(self, record: &RuntimeRecord) -> RuntimeRecord {
        let mut mismatched = record.clone();
        match self {
            Self::ControllerGeneration => {
                mismatched.controller_generation += FIXTURE_MISMATCH_INCREMENT;
            }
            Self::WorkerStartTime => mismatched.worker_start_time += FIXTURE_MISMATCH_INCREMENT,
            Self::ChildStartTime => mismatched.child_start_time += FIXTURE_MISMATCH_INCREMENT,
        }
        mismatched
    }
}

const TEST_SOCKET_NAME: &str = "custody.sock";
const TEST_TOKEN: &str = "host-custody-test-token";
const TEST_COMMAND: &str = "/bin/sh";
const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_STARTED_AT_MS: i64 = 1;
const TEST_DIGEST_BYTE: u8 = 0x5a;
#[cfg(target_os = "macos")]
const TEST_READER_HANDLE: u64 = 0x51;
#[cfg(target_os = "macos")]
const TEST_WRITER_HANDLE: u64 = 0x52;
#[cfg(target_os = "linux")]
const FOREIGN_READER_HANDLE: u64 = 0x61;
#[cfg(target_os = "linux")]
const FOREIGN_WRITER_HANDLE: u64 = 0x62;

#[derive(Clone, Copy)]
enum ReleaseResponse {
    ReadAcknowledgement,
    DropWithoutReading,
}

fn runtime(host: &Host, socket_path: std::path::PathBuf) -> RuntimeRecord {
    let process = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
    RuntimeRecord {
        schema_version: crate::constants::RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: process.pid,
        worker_start_time: process.start_time,
        child_pid: process.pid,
        child_start_time: process.start_time,
        boot_id: host.boot_id.clone(),
        socket_path,
        auth_token: TEST_TOKEN.to_owned(),
        controller_generation: host.controller_generation,
        started_at_ms: TEST_STARTED_AT_MS,
        command: TEST_COMMAND.to_owned(),
        args: Vec::new(),
        cwd: host.paths.home().to_owned(),
        cols: TEST_COLUMNS,
        rows: TEST_ROWS,
        bootstrap_delivery: BootstrapDelivery::Pending,
    }
}

fn persist_pending(host: &Host, record: &RuntimeRecord) -> CustodyDurableRecord {
    host.paths.ensure_session(record.terminal_id).unwrap();
    atomic_write_json(&host.paths.runtime_record(record.terminal_id), record).unwrap();
    let pending = CustodyDurableRecord::pending(CustodyRuntimeIdentity::from_record(record));
    atomic_write_json(&host.paths.pty_custody(record.terminal_id), &pending).unwrap();
    pending
}

#[cfg(target_os = "macos")]
fn persist_successor(host: &Host, record: &RuntimeRecord) -> CustodyDurableRecord {
    host.paths.ensure_session(record.terminal_id).unwrap();
    let guard = crate::paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
        &host.paths,
        &host.namespace,
        record.terminal_id,
    )
    .unwrap();
    let runtime_path = host.paths.runtime_record(record.terminal_id);
    let (_, runtime_identity): (RuntimeRecord, _) = guard
        .namespace()
        .read_json_with_identity(&runtime_path)
        .unwrap();
    guard
        .write_json_expected_with_commit_fence(&runtime_path, Some(runtime_identity), record)
        .unwrap();
    let pending = CustodyDurableRecord::pending(CustodyRuntimeIdentity::from_record(record));
    let custody_path = host.paths.pty_custody(record.terminal_id);
    let (_, custody_identity): (CustodyDurableRecord, _) = guard
        .namespace()
        .read_json_with_identity(&custody_path)
        .unwrap();
    guard
        .write_json_expected_with_commit_fence(&custody_path, Some(custody_identity), &pending)
        .unwrap();
    pending
}

#[cfg(target_os = "macos")]
fn persist_handshake_fixture(host: &Host, record: &RuntimeRecord) -> CustodyDurableRecord {
    let pending = persist_pending(host, record);
    finish_handshake_fixture(host, record, pending)
}

#[cfg(target_os = "macos")]
fn persist_successor_handshake_fixture(
    host: &Host,
    record: &RuntimeRecord,
) -> CustodyDurableRecord {
    let pending = persist_successor(host, record);
    finish_handshake_fixture(host, record, pending)
}

#[cfg(target_os = "macos")]
fn finish_handshake_fixture(
    host: &Host,
    record: &RuntimeRecord,
    pending: CustodyDurableRecord,
) -> CustodyDurableRecord {
    drop(
        crate::journal::Journal::open(host.paths.journal(record.terminal_id), record.runtime_epoch)
            .unwrap(),
    );
    let peer = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
    host.secondary_custody.authorize_peer_for_test(record, peer);
    pending
}

#[cfg(target_os = "macos")]
fn platform_descriptor_set() -> CustodyDescriptorSet {
    CustodyDescriptorSet::macos_process_capability(TEST_READER_HANDLE, TEST_WRITER_HANDLE)
}

#[cfg(target_os = "linux")]
fn platform_descriptor_set() -> CustodyDescriptorSet {
    CustodyDescriptorSet::LinuxContainment
}

#[cfg(target_os = "macos")]
fn mismatched_descriptor_set() -> CustodyDescriptorSet {
    CustodyDescriptorSet::LinuxContainment
}

#[cfg(target_os = "linux")]
fn mismatched_descriptor_set() -> CustodyDescriptorSet {
    CustodyDescriptorSet::macos_process_capability(FOREIGN_READER_HANDLE, FOREIGN_WRITER_HANDLE)
}

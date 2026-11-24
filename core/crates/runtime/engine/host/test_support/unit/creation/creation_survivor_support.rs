const SURVIVOR_TERMINATION_DETAIL: &str = "destructive authority is unavailable";
const GENERIC_TERMINATION_DETAIL: &str = "worker rejected cleanup termination";
const SINGLE_TERMINATION_REQUEST: usize = 1;

fn bind_private_cleanup_listener(path: &Path) -> UnixListener {
    use std::os::unix::fs::PermissionsExt;

    let listener = UnixListener::bind(path).unwrap();
    std::fs::set_permissions(
        path,
        std::fs::Permissions::from_mode(crate::paths::constants::PRIVATE_FILE_MODE),
    )
    .unwrap();
    listener
}

struct SurvivorCreationFixture {
    host: Host,
    creation_token: Uuid,
    binding: CreationBinding,
    runtime: RuntimeRecord,
    ledger_path: PathBuf,
    // Host cleanup needs its durable registration until exact socket withdrawal finishes.
    directory: tempfile::TempDir,
}

async fn survivor_creation_fixture(name: &str) -> SurvivorCreationFixture {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let (request, reserved, ledger_path) = reserve_test_creation(&host, directory.path(), name);
    host.prepare_bound_session(&reserved).await.unwrap();
    let launch = lock_recover(&host.creation_capabilities)
        .prepare_launch(&ledger_path, reserved.result)
        .unwrap();
    let binding = lock_recover(&host.creation_capabilities)
        .bindings
        .iter()
        .find(|entry| entry.result == reserved.result)
        .unwrap()
        .clone();
    let worker_launch = planned_worker_launch(&host, &binding, &launch);
    let runtime = runtime_for_creation(&host, &worker_launch, BootstrapDelivery::NotRequired, true);
    atomic_write_json(
        &host.paths.worker_launch(binding.result.terminal_id),
        &worker_launch,
    )
    .unwrap();
    atomic_write_json(
        &host.paths.runtime_record(binding.result.terminal_id),
        &runtime,
    )
    .unwrap();
    host.secondary_custody.authorize_rpc_test(&runtime);
    lock_recover(&host.creation_capabilities)
        .set_active_phase(&ledger_path, binding.result, CreationPhase::Ready)
        .unwrap();
    let binding = lock_recover(&host.creation_capabilities)
        .bindings
        .iter()
        .find(|entry| entry.result == reserved.result)
        .unwrap()
        .clone();
    SurvivorCreationFixture {
        host,
        creation_token: request.creation_token,
        binding,
        runtime,
        ledger_path,
        directory,
    }
}

fn cleanup_termination_request(binding: &CreationBinding) -> CreationCleanupTerminateRequest {
    CreationCleanupTerminateRequest {
        terminal_id: binding.result.terminal_id,
        creation_capability: binding.result.creation_capability,
        force: true,
    }
}

fn cleanup_discard_request(binding: &CreationBinding) -> CreationCleanupDiscardRequest {
    CreationCleanupDiscardRequest {
        terminal_id: binding.result.terminal_id,
        creation_capability: binding.result.creation_capability,
        force: true,
    }
}

fn cleanup_archive_request(binding: &CreationBinding) -> CreationCleanupArchiveRequest {
    CreationCleanupArchiveRequest {
        terminal_id: binding.result.terminal_id,
        creation_capability: binding.result.creation_capability,
    }
}

fn finalize_survivor_request(fixture: &SurvivorCreationFixture) -> FinalizeCreationRequest {
    FinalizeCreationRequest {
        creation_token: fixture.creation_token,
        terminal_id: fixture.binding.result.terminal_id,
        creation_capability: fixture.binding.result.creation_capability,
    }
}

fn survivor_error() -> ErrorBody {
    ErrorBody::new(
        crate::protocol::error_code::DESTRUCTIVE_SESSION_UNAVAILABLE,
        SURVIVOR_TERMINATION_DETAIL,
        false,
    )
}

fn durable_session_bytes(host: &Host, terminal_id: Uuid) -> (Vec<u8>, Vec<u8>, Option<Vec<u8>>) {
    (
        std::fs::read(host.paths.manifest(terminal_id)).unwrap(),
        std::fs::read(host.paths.runtime_record(terminal_id)).unwrap(),
        std::fs::read(host.paths.terminate_tombstone(terminal_id)).ok(),
    )
}

fn assert_survivor_completion(
    host: &Host,
    binding: &CreationBinding,
    ledger_path: &Path,
) -> CreationReceipt {
    assert_transition_ledger(ledger_path, &[]);
    let receipt = host
        .creation_receipts
        .lookup_binding(binding)
        .unwrap()
        .unwrap();
    assert_eq!(receipt.completion, CreationCompletion::Survivor);
    receipt
}

fn assert_uncertain_survivor_recovery(host: &Host, terminal_id: Uuid) {
    let durable: TerminalManifest = read_json(&host.paths.manifest(terminal_id)).unwrap();
    let catalog = lock_recover(&host.catalog)
        .get(terminal_id)
        .unwrap()
        .unwrap();
    assert_eq!(durable.recovery_state, RecoveryState::Uncertain);
    assert_eq!(catalog.recovery_state, RecoveryState::Uncertain);
}

include!("creation_survivor_server.rs");

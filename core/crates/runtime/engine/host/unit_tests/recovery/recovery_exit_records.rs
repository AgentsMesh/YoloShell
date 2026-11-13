use crate::host::tests::*;
use super::*;

const CORRUPT_EXIT_RECORD: &[u8] = b"{not-valid-json";
const DISPLACED_SESSIONS_DIRECTORY: &str = "displaced-sessions";
const TEST_EXIT_CODE: i32 = 0;
const TEST_LAST_OUTPUT_SEQUENCE: u64 = 0;
const TEST_WORKER_SOCKET: &str = "worker.sock";
const UNREADABLE_EXIT_RECORD_MODE: u32 = 0o000;

#[derive(Clone, Copy)]
enum ExitEvidence {
    Missing,
    Matching(ExitReason),
    Stale,
}

#[derive(Clone, Copy)]
struct ExitStateCase {
    evidence: ExitEvidence,
    expected_state: ExitRecordState,
    expected_restore: AutomaticRestore,
}

#[test]
fn exit_record_state_matrix_preserves_restore_contract() {
    let cases = [
        state_case(ExitEvidence::Missing, ExitRecordState::Missing, true),
        state_case(ExitEvidence::Stale, ExitRecordState::Stale, true),
        matching_case(ExitReason::WorkerCrash, true),
        matching_case(ExitReason::Natural, false),
        matching_case(ExitReason::UserTerminate, false),
        matching_case(ExitReason::HostReboot, false),
        matching_case(ExitReason::Lost, false),
    ];
    for case in cases {
        let (directory, host, manifest, runtime) = dead_runtime_fixture();
        persist_evidence(&host, &runtime, case.evidence);

        assert_eq!(host.exit_record_state(&runtime), case.expected_state);
        assert_eq!(
            host.automatic_restore_decision(&manifest),
            case.expected_restore
        );
        drop((host, directory));
    }
}

#[test]
fn corrupt_and_unreadable_exit_records_fail_closed() {
    for failure in [ExitFileFailure::Corrupt, ExitFileFailure::Unreadable] {
        let (directory, host, manifest, runtime) = dead_runtime_fixture();
        let path = host.paths.exit_record(runtime.terminal_id);
        failure.install(&path, &runtime);

        assert_eq!(host.exit_record_state(&runtime), ExitRecordState::Invalid);
        assert_eq!(
            host.automatic_restore_decision(&manifest),
            AutomaticRestore::Ineligible
        );
        drop((host, directory));
    }
}

#[test]
fn namespace_failure_makes_exit_evidence_invalid() {
    let (directory, host, _manifest, runtime) = dead_runtime_fixture();
    persist_evidence(
        &host,
        &runtime,
        ExitEvidence::Matching(ExitReason::WorkerCrash),
    );
    let sessions = host.paths.sessions_dir();
    let displaced = directory.path().join(DISPLACED_SESSIONS_DIRECTORY);
    std::fs::rename(&sessions, &displaced).unwrap();
    crate::paths::directory::PrivateDirectory::ensure(&sessions).unwrap();

    assert_eq!(host.exit_record_state(&runtime), ExitRecordState::Invalid);
    assert!(!host.exit_allows_restore(&runtime));
    assert!(host.namespace.verify().is_err());
}

fn dead_runtime_fixture() -> (tempfile::TempDir, Host, TerminalManifest, RuntimeRecord) {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let mut runtime = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join(TEST_WORKER_SOCKET),
    );
    install_reaped_runtime_processes(&mut runtime);
    persist_host_session(&host, &manifest, &runtime);
    host.secondary_custody
        .install_test_retirement_evidence(&host, &runtime);
    (directory, host, manifest, runtime)
}

fn matching_case(reason: ExitReason, allows_restore: bool) -> ExitStateCase {
    state_case(
        ExitEvidence::Matching(reason),
        ExitRecordState::Matching(reason),
        allows_restore,
    )
}

fn state_case(
    evidence: ExitEvidence,
    expected_state: ExitRecordState,
    allows_restore: bool,
) -> ExitStateCase {
    ExitStateCase {
        evidence,
        expected_state,
        expected_restore: if allows_restore {
            AutomaticRestore::Eligible
        } else {
            AutomaticRestore::Ineligible
        },
    }
}

fn persist_evidence(host: &Host, runtime: &RuntimeRecord, evidence: ExitEvidence) {
    let exit = match evidence {
        ExitEvidence::Missing => return,
        ExitEvidence::Matching(reason) => exit_record(runtime.runtime_epoch, reason),
        ExitEvidence::Stale => exit_record(stale_epoch(runtime.runtime_epoch), ExitReason::Natural),
    };
    atomic_write_json(&host.paths.exit_record(runtime.terminal_id), &exit).unwrap();
}

fn stale_epoch(current: Uuid) -> Uuid {
    loop {
        let candidate = Uuid::new_v4();
        if candidate != current {
            return candidate;
        }
    }
}

pub(super) fn exit_record(runtime_epoch: Uuid, reason: ExitReason) -> ExitRecord {
    ExitRecord {
        runtime_epoch,
        code: Some(TEST_EXIT_CODE),
        signal: None,
        reason,
        observed_at_ms: now_ms(),
        last_output_sequence: TEST_LAST_OUTPUT_SEQUENCE,
    }
}

#[derive(Clone, Copy)]
enum ExitFileFailure {
    Corrupt,
    Unreadable,
}

impl ExitFileFailure {
    fn install(self, path: &Path, runtime: &RuntimeRecord) {
        match self {
            Self::Corrupt => write_private_bytes(path, CORRUPT_EXIT_RECORD),
            Self::Unreadable => {
                atomic_write_json(
                    path,
                    &exit_record(runtime.runtime_epoch, ExitReason::Natural),
                )
                .unwrap();
                std::fs::set_permissions(
                    path,
                    <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(
                        UNREADABLE_EXIT_RECORD_MODE,
                    ),
                )
                .unwrap();
            }
        }
    }
}

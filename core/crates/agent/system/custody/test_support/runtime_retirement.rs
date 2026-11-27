use super::*;
use crate::claude_custody::{
    CustodySpec,
    types::{CustodyRecord, LEDGER_SCHEMA_VERSION},
};
use crate::model::{BootstrapDelivery, RecoveryState, RuntimeRecord, TerminalManifest};
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use yolo_persistence_custody::{journal::Journal, pty_custody::*};

const TEST_NAMESPACE: &str = "Claude runtime retirement test";
const TEST_COMMAND: &str = "/bin/sh";
const TEST_TOKEN: &str = "runtime-retirement-fixture-token";
const TEST_GENERATION: u64 = 1;
const TEST_TIMESTAMP: i64 = 1;
const TEST_COLUMNS: u16 = 80;
const TEST_ROWS: u16 = 24;
const GUARDIAN_PID: u32 = 960_001;
const STANDBY_PID: u32 = 960_002;
const GUARDIAN_START: u64 = 970_001;
const TEST_SOCKET: &str = "retirement.sock";

pub(super) struct Fixture {
    _root: tempfile::TempDir,
    pub(super) paths: Paths,
    pub(super) namespace: NamespaceLease,
    pub(super) closed: crate::process::PreparedRetirementCustodyFixture,
    pub(super) armed: CustodyDurableRecord,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let closed = crate::process::prepared_retirement_custody_fixture();
        let root = tempfile::tempdir().unwrap();
        let paths = Paths::new(root.path().canonicalize().unwrap());
        let runtime = &closed.runtime;
        paths.ensure_session(runtime.terminal_id).unwrap();
        let session = paths.session_dir(runtime.terminal_id);
        let namespace =
            NamespaceLease::capture(&[(TEST_NAMESPACE, paths.home()), (TEST_NAMESPACE, &session)])
                .unwrap();
        let manifest = TerminalManifest {
            schema_version: crate::constants::SESSION_MANIFEST_SCHEMA_VERSION,
            terminal_id: runtime.terminal_id,
            name: TEST_NAMESPACE.into(),
            command: TEST_COMMAND.into(),
            args: Vec::new(),
            cwd: paths.home().to_owned(),
            cols: TEST_COLUMNS,
            rows: TEST_ROWS,
            created_at_ms: TEST_TIMESTAMP,
            updated_at_ms: TEST_TIMESTAMP,
            archived: false,
            recovery_state: RecoveryState::Created,
        };
        namespace
            .write_json(&paths.manifest(runtime.terminal_id), &manifest)
            .unwrap();
        let record = RuntimeRecord {
            schema_version: crate::constants::RUNTIME_RECORD_SCHEMA_VERSION,
            terminal_id: runtime.terminal_id,
            runtime_epoch: runtime.runtime_epoch,
            worker_id: runtime.worker_id,
            worker_pid: runtime.worker_pid,
            worker_start_time: runtime.worker_start_time,
            child_pid: runtime.child_pid,
            child_start_time: runtime.child_start_time,
            boot_id: runtime.boot_id.clone(),
            socket_path: paths.home().join(TEST_SOCKET),
            auth_token: TEST_TOKEN.into(),
            controller_generation: TEST_GENERATION,
            started_at_ms: TEST_TIMESTAMP,
            command: TEST_COMMAND.into(),
            args: Vec::new(),
            cwd: paths.home().to_owned(),
            cols: TEST_COLUMNS,
            rows: TEST_ROWS,
            bootstrap_delivery: BootstrapDelivery::Delivered,
        };
        namespace
            .write_json(&paths.runtime_record(runtime.terminal_id), &record)
            .unwrap();
        drop(Journal::open(&paths.journal(runtime.terminal_id), runtime.runtime_epoch).unwrap());
        let process = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
        let holder = CustodyHolderIdentity {
            pid: process.pid,
            start_time: process.start_time,
        };
        let request = CustodyRequest::new(
            CustodyLeaseId::new(),
            runtime.clone(),
            holder,
            TEST_TOKEN.into(),
            TEST_GENERATION,
        );
        let offer = CustodyOffer::new_with_profile(
            &request,
            closed.descriptor_set,
            CustodyProfile::ActiveRunning,
            closed.digest,
        )
        .unwrap();
        let binding =
            CustodyGuardianBinding::new(uuid::Uuid::new_v4(), holder, &request, &offer).unwrap();
        let armed = CustodyDurableRecord::pending(runtime.clone())
            .prepare_guardian(&request, &offer, binding)
            .unwrap()
            .arm_prepared_guardian(&request, &offer, binding)
            .unwrap();
        namespace
            .write_json(&paths.pty_custody(runtime.terminal_id), &armed)
            .unwrap();
        Self {
            _root: root,
            paths,
            namespace,
            closed,
            armed,
        }
    }

    pub(super) fn ledger_path(&self) -> std::path::PathBuf {
        self.paths
            .session_dir(self.closed.runtime.terminal_id)
            .join(crate::claude_custody::LEDGER_FILE)
    }

    pub(super) fn ledger(&self) -> CustodyLedger {
        self.namespace.read_json(&self.ledger_path()).unwrap()
    }

    pub(super) fn install_ledger(&self) -> CustodyLedger {
        static NEXT_START: std::sync::atomic::AtomicU64 =
            std::sync::atomic::AtomicU64::new(GUARDIAN_START);
        let guardian_start =
            NEXT_START.fetch_add(NEXT_GENERATION, std::sync::atomic::Ordering::Relaxed);
        let runtime = &self.closed.runtime;
        let identity = |pid, start_time| DurableProcessIdentity {
            boot_id: runtime.boot_id.clone(),
            pid,
            start_time,
        };
        let ledger = CustodyLedger {
            schema_version: LEDGER_SCHEMA_VERSION,
            records: vec![CustodyRecord {
                spec: CustodySpec {
                    invocation_id: uuid::Uuid::new_v4(),
                    worker: identity(runtime.worker_pid, runtime.worker_start_time),
                    target: identity(runtime.child_pid, runtime.child_start_time),
                    guardians: [
                        identity(GUARDIAN_PID, guardian_start),
                        identity(STANDBY_PID, guardian_start),
                    ],
                },
                state: CustodyState::Indeterminate,
                publication: CustodyPublication::Active,
            }],
        };
        self.namespace
            .write_json(&self.ledger_path(), &ledger)
            .unwrap();
        ledger
    }

    pub(super) fn current(&self) -> CustodyDurableRecord {
        self.namespace
            .read_json(&self.paths.pty_custody(self.closed.runtime.terminal_id))
            .unwrap()
    }

    pub(super) fn publish(
        &self,
        settle: impl FnOnce(&VerifiedRuntimeRetirement<'_, '_>) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        let guard = yolo_paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
            &self.paths,
            &self.namespace,
            self.closed.runtime.terminal_id,
        )?;
        crate::custody_retirement::publish_exact_with_guard(
            &guard,
            &self.paths,
            &self.closed.custody,
            &self.closed.runtime,
            self.armed.armed_lease().unwrap(),
            settle,
        )
    }
}

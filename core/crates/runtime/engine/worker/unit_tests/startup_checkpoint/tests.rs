use super::constants::{SANITIZED_FENCE_FIELDS, SANITIZED_FIELDS, SUCCESSOR_FIELD, UNKNOWN_FIELD};
use super::*;
use crate::constants::WORKER_LAUNCH_SCHEMA_VERSION;
use crate::model::WorkerLaunchSpec;
use crate::paths::Paths;
use crate::worker::WorkerNamespace;
use std::collections::BTreeMap;
use std::path::PathBuf;
use uuid::Uuid;

#[path = "adversarial_tests.rs"]
mod adversarial_tests;
#[path = "../../test_support/unit/startup_checkpoint/authority_test_support.rs"]
mod authority_test_support;
#[path = "authority_tests.rs"]
mod authority_tests;
#[path = "boot_custody_tests.rs"]
mod boot_custody_tests;
#[path = "custody_publication_identity_tests.rs"]
mod custody_publication_identity_tests;
#[path = "custody_rollover_tests.rs"]
mod custody_rollover_tests;
#[path = "custody_rollover_wire_tests.rs"]
mod custody_rollover_wire_tests;
#[path = "../../test_support/unit/startup_checkpoint/durable_version.rs"]
mod durable_version;
#[path = "facade_tests.rs"]
mod facade_tests;
#[path = "lifecycle_mutation_tests.rs"]
mod lifecycle_mutation_tests;
#[path = "mutation_fence_tests.rs"]
mod mutation_fence_tests;
#[path = "path_io_tests.rs"]
mod path_io_tests;
#[path = "reboot_validation_tests.rs"]
mod reboot_validation_tests;
#[path = "replay_disposition_tests.rs"]
mod replay_disposition_tests;
#[path = "retained_runtime_tests.rs"]
mod retained_runtime_tests;
#[path = "retirement_adversarial_tests.rs"]
mod retirement_adversarial_tests;
#[path = "../../test_support/unit/startup_checkpoint/retirement_test_support.rs"]
mod retirement_test_support;
#[path = "retirement_tests.rs"]
mod retirement_tests;
#[path = "runtime_publication_tests.rs"]
mod runtime_publication_tests;
#[path = "schema_rejection_tests.rs"]
mod schema_rejection_tests;
#[path = "settlement_tests.rs"]
mod settlement_tests;
#[path = "startup_lock_wait_tests.rs"]
mod startup_lock_wait_tests;
#[path = "transition_tests.rs"]
mod transition_tests;

const TEST_AUTH_TOKEN: &str = "3f80cb39eaf14ca2a9adfb77dc0f9cb9";
const TEST_COMMAND: &str = "/private/secret/bin/private-shell";
const TEST_ARGUMENT: &str = "private-command-argument";
const TEST_CWD: &str = "/private/secret/working-directory";
const TEST_ENVIRONMENT_KEY: &str = "PRIVATE_PROVIDER_TOKEN";
const TEST_ENVIRONMENT_VALUE: &str = "private-provider-value";
const TEST_INITIAL_INPUT: &[u8] = b"private initial input\n";
const TEST_HOLD_COMMAND: &str = "/bin/sleep";
const TEST_HOLD_DURATION_ARGUMENT: &str = "30";
const TEST_INVALID_CHECKPOINT_JSON: &[u8] = b"{not-json";
const TEST_CONTROLLER_GENERATION: u64 = 7;
const INVALID_CONTROLLER_GENERATION: u64 = 0;
const INVALID_PROCESS_ID: u32 = 0;
const TEST_COLUMNS: u16 = 80;
const TEST_ROWS: u16 = 24;
const TEST_CHILD_PID_PRIMARY: u32 = 2;
const TEST_CHILD_PID_FALLBACK: u32 = 3;
const TEST_CHILD_START_TIME: u64 = 11;
const TEST_STARTED_AT_MS: i64 = 1;
const TEST_OTHER_BOOT_ID: &str = "test-other-boot-id";
const TEST_CURRENT_BOOT_ID: &str = "test-current-boot-id";
const TEST_UNKNOWN_BOOT_SUFFIX: &str = "fixture";
const CHILD_BINDING_STAGE_COUNT: usize = 6;
const STALE_IDENTITY_OFFSET: u64 = 1;
const UNSUPPORTED_SCHEMA_VERSION_OFFSET: u16 = 1;
const OVERSIZE_SENTINEL_BYTES: usize = 1;

const STARTUP_STAGES: [StartupStage; 12] = [
    StartupStage::StartupContainment,
    StartupStage::WorkerSocket,
    StartupStage::AgentRuntime,
    StartupStage::TerminalJournal,
    StartupStage::Pty,
    StartupStage::ChildProcess,
    StartupStage::BootstrapGate,
    StartupStage::RuntimeRecord,
    StartupStage::SecondaryCustody,
    StartupStage::CommandExec,
    StartupStage::BootstrapInput,
    StartupStage::RuntimeActivation,
];

struct CheckpointFixture {
    _root: tempfile::TempDir,
    paths: Paths,
    namespace: WorkerNamespace,
    launch: WorkerLaunchSpec,
    worker: ProcessIdentity,
}

impl CheckpointFixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let paths = Paths::new(root.path());
        let terminal_id = Uuid::new_v4();
        paths.ensure_session(terminal_id).unwrap();
        let namespace = WorkerNamespace::capture(&paths, terminal_id).unwrap();
        let launch = WorkerLaunchSpec {
            schema_version: WORKER_LAUNCH_SCHEMA_VERSION,
            terminal_id,
            runtime_epoch: Uuid::new_v4(),
            worker_id: Uuid::new_v4(),
            socket_path: paths.worker_socket(terminal_id).unwrap(),
            auth_token: TEST_AUTH_TOKEN.into(),
            controller_generation: TEST_CONTROLLER_GENERATION,
            command: TEST_COMMAND.into(),
            args: vec![TEST_ARGUMENT.into()],
            cwd: PathBuf::from(TEST_CWD),
            environment: BTreeMap::from([(
                TEST_ENVIRONMENT_KEY.into(),
                TEST_ENVIRONMENT_VALUE.into(),
            )]),
            initial_input: Some(TEST_INITIAL_INPUT.into()),
            cols: TEST_COLUMNS,
            rows: TEST_ROWS,
            context: None,
        };
        crate::paths::atomic_write_json(&paths.worker_launch(terminal_id), &launch).unwrap();
        let worker = ProcessIdentity::capture(std::process::id()).unwrap();
        Self {
            _root: root,
            paths,
            namespace,
            launch,
            worker,
        }
    }

    fn begin(&self) -> ActiveStartupCheckpoint {
        ActiveStartupCheckpoint::begin(&self.namespace, &self.paths, &self.launch, self.worker)
            .unwrap()
    }

    fn path(&self) -> PathBuf {
        self.paths
            .worker_startup_checkpoint(self.launch.terminal_id)
    }

    fn read(&self) -> StartupCheckpointEvidence {
        let (evidence, _) = io::read(self.namespace.file(&self.path()).unwrap(), || {
            self.namespace.verify()
        })
        .unwrap();
        validation::validate(&evidence).unwrap();
        evidence
    }

    fn child(&self) -> ProcessIdentity {
        let pid = if self.worker.pid == TEST_CHILD_PID_PRIMARY {
            TEST_CHILD_PID_FALLBACK
        } else {
            TEST_CHILD_PID_PRIMARY
        };
        ProcessIdentity {
            pid,
            start_time: TEST_CHILD_START_TIME,
        }
    }

    fn write_bytes(&self, bytes: &[u8], max_bytes: u64) {
        crate::paths::write_private_bytes_guarded(
            self.namespace.file(&self.path()).unwrap(),
            bytes,
            max_bytes,
            || self.namespace.verify(),
        )
        .unwrap();
    }
}

fn gone_process() -> ProcessIdentity {
    let mut process = crate::process::spawn_waitable_test_child(
        std::process::Command::new(TEST_HOLD_COMMAND).arg(TEST_HOLD_DURATION_ARGUMENT),
    );
    let identity = ProcessIdentity::capture(process.id()).unwrap();
    process.kill().unwrap();
    process.wait().unwrap();
    identity
}

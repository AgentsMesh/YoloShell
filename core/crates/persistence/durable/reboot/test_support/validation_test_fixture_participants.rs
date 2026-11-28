use super::super::ParticipantTargets;
use super::test_fixture_records::{CHILD_PID, WORKER_PID, WORKER_START};
use crate::agent::claude::generation::{
    AuthorityProcessIdentity, ClaudeInvocationAuthorities, GenerationBegin,
};
use crate::claude_custody::{CustodyLedger, CustodySpec, DurableProcessIdentity};
use crate::model::RuntimeRecord;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::pty_custody::CustodyDurableRecord;
use uuid::Uuid;

const AUTHORITY_PROCESS_PID: u32 = 75;
const AUTHORITY_PROCESS_START: u64 = 76;
const CLAUDE_DATA_HOME_DIRECTORY: &str = "claude-data-home";
const FIRST_GUARDIAN_PID_OFFSET: u32 = 10;
const GENERATION_SETTINGS_FILE: &str = "reboot-generation-settings";
const HANDSHAKE_DEADLINE_NS: u64 = 1;
const IDENTITY_START_OFFSET: u64 = 100;
const SECOND_GUARDIAN_PID_OFFSET: u32 = 11;
const SETTINGS_CONTENT: &[u8] = b"typed reboot settings";
const TARGET_PID_OFFSET: u32 = 0;
const ACTIVE_IS_FOREGROUND: bool = false;

pub(super) fn targets(
    mut runtime: RuntimeRecord,
    mut checkpoint: crate::worker::StartupCheckpointEvidence,
    mut custody: CustodyLedger,
    mut generation: ClaudeInvocationAuthorities,
    mut pty: CustodyDurableRecord,
    target_boot: &str,
) -> ParticipantTargets {
    checkpoint.boot_id = target_boot.to_owned();
    crate::claude_custody::reboot_test_hook::rewrite_ledger_boot(
        &mut custody,
        &runtime.boot_id,
        target_boot,
    )
    .unwrap();
    crate::agent::claude::generation_test_hook::rewrite_set_boot(
        &mut generation,
        runtime.runtime_epoch,
        &runtime.boot_id,
        target_boot,
    )
    .unwrap();
    crate::pty_custody::rewrite_boot_for_test(&mut pty, &runtime, target_boot).unwrap();
    runtime.boot_id = target_boot.to_owned();
    ParticipantTargets {
        startup_checkpoint: checkpoint,
        claude_custody: Some(custody),
        claude_generation: Some(generation),
        pty_custody: Some(pty),
        runtime,
    }
}

pub(super) fn generation(
    runtime: &RuntimeRecord,
    namespace: &NamespaceLease,
    paths: &Paths,
) -> ClaudeInvocationAuthorities {
    let store = crate::agent::claude::generation::ClaudeGenerationStore::from_binding_path(
        &paths.agent_binding(runtime.terminal_id),
    )
    .unwrap();
    let settings = paths
        .session_dir(runtime.terminal_id)
        .join(GENERATION_SETTINGS_FILE);
    let settings_identity = crate::paths::private_file::PrivateFileEntry::open(&settings)
        .unwrap()
        .create_bytes_exclusive(
            SETTINGS_CONTENT,
            crate::claude_hooks::MAX_CLAUDE_SETTINGS_BYTES,
        )
        .unwrap();
    let begin = GenerationBegin {
        terminal_id: runtime.terminal_id,
        runtime_epoch: runtime.runtime_epoch,
        invocation_id: Uuid::new_v4(),
        hook_token: Uuid::new_v4(),
        helper_capability: Uuid::new_v4(),
        worker: AuthorityProcessIdentity {
            pid: WORKER_PID,
            start_time: WORKER_START,
        },
        process: AuthorityProcessIdentity {
            pid: AUTHORITY_PROCESS_PID,
            start_time: AUTHORITY_PROCESS_START,
        },
        settings_identity,
        handshake_boot_id: runtime.boot_id.clone(),
        handshake_deadline_ns: HANDSHAKE_DEADLINE_NS,
        provider_session_id: Uuid::new_v4().hyphenated().to_string(),
        discover_session: false,
        mode: crate::agent::claude::generation::InvocationMode::Fresh,
        cwd: runtime.cwd.clone(),
        data_home: runtime.cwd.join(CLAUDE_DATA_HOME_DIRECTORY),
    };
    store
        .begin_foreground_in_namespace(namespace, &begin, ACTIVE_IS_FOREGROUND)
        .unwrap();
    namespace.read_json(store.path()).unwrap()
}

pub(super) fn custody(runtime: &RuntimeRecord, fixture_path: &std::path::Path) -> CustodyLedger {
    let identity = |offset: u32| DurableProcessIdentity {
        boot_id: runtime.boot_id.clone(),
        pid: CHILD_PID + offset,
        start_time: IDENTITY_START_OFFSET + u64::from(offset),
    };
    let spec = CustodySpec {
        invocation_id: Uuid::new_v4(),
        target: identity(TARGET_PID_OFFSET),
        worker: DurableProcessIdentity {
            boot_id: runtime.boot_id.clone(),
            pid: WORKER_PID,
            start_time: WORKER_START,
        },
        guardians: [
            identity(FIRST_GUARDIAN_PID_OFFSET),
            identity(SECOND_GUARDIAN_PID_OFFSET),
        ],
    };
    crate::claude_custody::arm(fixture_path, &spec).unwrap();
    crate::paths::read_json(fixture_path).unwrap()
}

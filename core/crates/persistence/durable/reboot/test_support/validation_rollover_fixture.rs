use super::super::{
    OptionalParticipant, SessionDurableMutationGuard, adversarial_targets, prepare_intent,
};
use super::{RebootMigrationIntent, test_fixture::Fixture};
use crate::durable_reboot::durable_test_io::DurableTestRemoval;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::pty_custody::{CustodyDurableRecord, CustodyRuntimeIdentity};
use crate::sha256_digest::Sha256Digest;
use crate::worker::StartupPtyCustodyRollover;
use crate::worker::constants::STARTUP_CHECKPOINT_SCHEMA_VERSION;
use uuid::Uuid;

const CHILD_PID_OFFSET: u32 = 12;
const CHILD_START_OFFSET: u64 = 14;
const NAMESPACE_LABEL: &str = "durable reboot rollover validation";
const PREDECESSOR_DIGEST_BYTE: u8 = 0x6d;
const WORKER_PID_OFFSET: u32 = 11;
const WORKER_START_OFFSET: u64 = 13;

pub(in crate::durable_reboot) struct RolloverFixture {
    pub(in crate::durable_reboot) _base: Fixture,
    pub(in crate::durable_reboot) intent: RebootMigrationIntent,
    pub(in crate::durable_reboot) source_custody: CustodyDurableRecord,
}

impl RolloverFixture {
    pub(in crate::durable_reboot) fn new() -> Self {
        let base = Fixture::new();
        let paths = Paths::new(base._root.path());
        let session = paths.session_dir(base.intent.terminal_id);
        let namespace = NamespaceLease::capture(&[(NAMESPACE_LABEL, &session)]).unwrap();
        let guard =
            SessionDurableMutationGuard::acquire(&paths, &namespace, base.intent.terminal_id)
                .unwrap();
        discard_staged_intent(&paths, &guard, &base.intent);
        super::super::preparation_recovery::recover_expected_orphan(
            &paths,
            &guard,
            base.intent.preparation_authority_version,
        )
        .unwrap();
        let mut runtime = base.intent.runtime.target_payload.clone();
        runtime.boot_id.clone_from(&base.intent.source_boot_id);
        let source_custody = predecessor_custody(&runtime);
        install_rollover(&paths, &namespace, &runtime, &source_custody);
        let targets =
            adversarial_targets::capture(&paths, &guard, &runtime, &base.intent.target_boot_id)
                .unwrap();
        let intent = prepare_intent(&paths, &guard, targets).unwrap();
        drop(guard);
        Self {
            _base: base,
            intent,
            source_custody,
        }
    }
}

fn discard_staged_intent(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    intent: &RebootMigrationIntent,
) {
    let paths =
        super::super::participant_paths::ParticipantPaths::new(paths, intent.terminal_id).unwrap();
    discard_optional(
        guard,
        &paths.startup_checkpoint,
        intent,
        super::super::constants::STARTUP_CHECKPOINT_PARTICIPANT,
        &intent.startup_checkpoint,
    );
    discard_optional(
        guard,
        &paths.claude_custody,
        intent,
        super::super::constants::CLAUDE_CUSTODY_PARTICIPANT,
        &intent.claude_custody,
    );
    discard_optional(
        guard,
        &paths.claude_generation,
        intent,
        super::super::constants::CLAUDE_GENERATION_PARTICIPANT,
        &intent.claude_generation,
    );
    discard_optional(
        guard,
        &paths.pty_custody,
        intent,
        super::super::constants::PTY_CUSTODY_PARTICIPANT,
        &intent.pty_custody,
    );
    discard_required(
        guard,
        &paths.runtime,
        intent,
        super::super::constants::RUNTIME_PARTICIPANT,
        &intent.runtime,
    );
}

fn discard_optional<T>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &std::path::Path,
    intent: &RebootMigrationIntent,
    name: &str,
    participant: &OptionalParticipant<T>,
) {
    if let OptionalParticipant::Replace(required) = participant {
        discard_required(guard, path, intent, name, required);
    }
}

fn discard_required<T>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &std::path::Path,
    intent: &RebootMigrationIntent,
    name: &str,
    participant: &super::super::schema::RequiredParticipant<T>,
) {
    super::super::staging::discard_prepared(
        guard,
        path,
        intent.transaction_id,
        name,
        participant.target_version,
    )
    .unwrap();
    let prepared =
        super::super::receipt::prepared_path(guard, path, intent.transaction_id, name).unwrap();
    prepared
        .remove_if_durable_exact(
            participant.prepared_receipt_version,
            crate::paths::constants::MAX_DURABLE_JSON_BYTES,
        )
        .unwrap();
}

fn install_rollover(
    paths: &Paths,
    namespace: &NamespaceLease,
    runtime: &crate::model::RuntimeRecord,
    custody: &CustodyDurableRecord,
) {
    let checkpoint_path = paths.worker_startup_checkpoint(runtime.terminal_id);
    let mut checkpoint: crate::worker::StartupCheckpointEvidence =
        namespace.read_json(&checkpoint_path).unwrap();
    checkpoint.schema_version = STARTUP_CHECKPOINT_SCHEMA_VERSION;
    checkpoint.predecessor_runtime_sha256 = Some(Sha256Digest::repeated(PREDECESSOR_DIGEST_BYTE));
    let custody_path = paths.pty_custody(runtime.terminal_id);
    namespace.write_json(&custody_path, custody).unwrap();
    let (_, _, source_version): (CustodyDurableRecord, _, _) =
        namespace.read_json_with_versions(&custody_path).unwrap();
    checkpoint.pending_pty_custody_rollover = Some(StartupPtyCustodyRollover {
        source_version,
        source_sha256: crate::pty_custody::durable_record_digest(custody).unwrap(),
    });
    namespace.write_json(&checkpoint_path, &checkpoint).unwrap();
}

fn predecessor_custody(runtime: &crate::model::RuntimeRecord) -> CustodyDurableRecord {
    let mut identity = CustodyRuntimeIdentity::from_record(runtime);
    identity.runtime_epoch = Uuid::new_v4();
    identity.worker_id = Uuid::new_v4();
    identity.worker_pid += WORKER_PID_OFFSET;
    identity.worker_start_time += WORKER_START_OFFSET;
    identity.child_pid += CHILD_PID_OFFSET;
    identity.child_start_time += CHILD_START_OFFSET;
    CustodyDurableRecord::pending(identity)
}

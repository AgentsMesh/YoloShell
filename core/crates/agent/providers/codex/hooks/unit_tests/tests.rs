use super::*;
use crate::agent::hooks::{
    EventKind, EventSequence, Generation, HookEnvelope, HookIdentity, ProcessIdentity, HookProviderKind,
};
use uuid::Uuid;

const TEST_PROVIDER_SESSION: &str = "thread-1";
const TEST_TURN: &str = "turn-1";
const TEST_CWD: &str = "/workspace";
const TEST_SESSION_ROOT: &str = "/tmp/yolo-codex-tests";
const TEST_PROCESS_PID: u32 = 31;
const TEST_PROCESS_START: u64 = 41;

fn identity() -> CodexHookIdentity {
    let session_id = Uuid::new_v4();
    let invocation_id = Uuid::new_v4();
    let paths = crate::paths::Paths::new(TEST_SESSION_ROOT);
    CodexHookIdentity::for_test(
        &paths,
        HookIdentity {
            terminal_id: session_id,
            runtime_epoch: Uuid::new_v4(),
            invocation_id,
            hook_token: Uuid::new_v4(),
            process: ProcessIdentity::new(TEST_PROCESS_PID, TEST_PROCESS_START),
            generation: Generation::INITIAL,
        },
        ACCEPTED_PROVIDER_VERSION,
    )
    .unwrap()
}

fn envelope(identity: &CodexHookIdentity, id: Uuid) -> HookEnvelope {
    HookEnvelope {
        schema_version: crate::agent::hooks::CURRENT_SCHEMA_VERSION,
        event_id: id,
        sequence: EventSequence::ZERO,
        provider: HookProviderKind::Codex,
        provider_version: identity.provider_version,
        identity: identity.hook.clone(),
        provider_session_id: TEST_PROVIDER_SESSION.to_owned(),
        turn_id: Some(TEST_TURN.to_owned()),
        agent_id: None,
        tool_name: None,
        tool_use_id: None,
        stop_hook_active: None,
        metadata: Default::default(),
        event: EventKind::UserPromptSubmit,
    }
}

#[path = "admission/admission_orphan_adversarial_tests.rs"]
mod admission_orphan_adversarial_tests;
#[path = "admission/admission_orphan_recovery_tests.rs"]
mod admission_orphan_recovery_tests;
#[path = "../test_support/admission_orphan.rs"]
mod admission_orphan_test_support;
#[path = "admission/admission_scan_tests.rs"]
mod admission_scan_tests;
#[path = "../test_support/admission.rs"]
mod admission_test_support;
#[path = "artifacts_tests.rs"]
mod artifacts_tests;
#[path = "intake/intake_budget_tests.rs"]
mod intake_budget_tests;
#[path = "intake/intake_envelope_validation_tests.rs"]
mod intake_envelope_validation_tests;
#[path = "intake/intake_isolation_tests.rs"]
mod intake_isolation_tests;
#[path = "intake/intake_native_tests.rs"]
mod intake_native_tests;
#[path = "intake/intake_persistence_tests.rs"]
mod intake_persistence_tests;
#[path = "intake/intake_scan_tests.rs"]
mod intake_scan_tests;
#[path = "../test_support/intake.rs"]
mod intake_test_support;
#[path = "mapping_contract_tests.rs"]
mod mapping_contract_tests;
#[path = "outbox/outbox_authority_tests.rs"]
mod outbox_authority_tests;
#[path = "outbox/outbox_competing_helper_tests.rs"]
mod outbox_competing_helper_tests;
#[path = "outbox/outbox_envelope_validation_tests.rs"]
mod outbox_envelope_validation_tests;
#[path = "outbox/outbox_history_validation_tests.rs"]
mod outbox_history_validation_tests;
#[path = "outbox/outbox_lock_baseline_tests.rs"]
mod outbox_lock_baseline_tests;
#[path = "outbox/outbox_occurrence_capacity_tests.rs"]
mod outbox_occurrence_capacity_tests;
#[path = "outbox/outbox_pending_witness_tests.rs"]
mod outbox_pending_witness_tests;
#[path = "outbox/outbox_receipt_sweep_tests.rs"]
mod outbox_receipt_sweep_tests;
#[path = "../test_support/outbox_receipt.rs"]
mod outbox_receipt_test_support;
#[path = "outbox/outbox_receipt_tests.rs"]
mod outbox_receipt_tests;
#[path = "outbox/outbox_rejection_tests.rs"]
mod outbox_rejection_tests;
#[path = "outbox/outbox_storage_reproof_tests.rs"]
mod outbox_storage_reproof_tests;
#[path = "outbox/outbox_tests.rs"]
mod outbox_tests;
#[path = "outbox/outbox_transaction_tests.rs"]
mod outbox_transaction_tests;
#[path = "retirement/retirement_directory_lineage_adversarial_tests.rs"]
mod retirement_directory_lineage_adversarial_tests;
#[path = "retirement/retirement_fence_thread_tests.rs"]
mod retirement_fence_thread_tests;
#[path = "retirement/retirement_record_adversarial_tests.rs"]
mod retirement_record_adversarial_tests;
#[path = "retirement/retirement_seal_adversarial_tests.rs"]
mod retirement_seal_adversarial_tests;
#[path = "retirement/retirement_seal_tests.rs"]
mod retirement_seal_tests;
#[path = "retirement/retirement_state_adversarial_tests.rs"]
mod retirement_state_adversarial_tests;
#[path = "retirement/retirement_state_tests.rs"]
mod retirement_state_tests;
#[path = "../test_support/retirement_terminal.rs"]
mod retirement_terminal_test_support;
#[path = "../test_support/retirement_constants.rs"]
mod retirement_test_constants;
#[path = "../test_support/retirement.rs"]
mod retirement_test_support;
#[path = "retirement/retirement_writer_adversarial_tests.rs"]
mod retirement_writer_adversarial_tests;
#[path = "retirement/retirement_writer_tests.rs"]
mod retirement_writer_tests;
#[path = "snapshot/snapshot_authority_tests.rs"]
mod snapshot_authority_tests;
#[path = "snapshot/snapshot_claim_publication_tests.rs"]
mod snapshot_claim_publication_tests;
#[path = "snapshot/snapshot_claim_tests.rs"]
mod snapshot_claim_tests;
#[path = "snapshot/snapshot_cursor_tests.rs"]
mod snapshot_cursor_tests;
#[path = "snapshot/snapshot_fence_classification_tests.rs"]
mod snapshot_fence_classification_tests;
#[path = "../test_support/snapshot.rs"]
mod snapshot_test_support;
#[path = "snapshot/snapshot_transaction_tests.rs"]
mod snapshot_transaction_tests;
#[path = "wire/wire_contract_tests.rs"]
mod wire_contract_tests;
#[path = "wire/wire_metadata_tests.rs"]
mod wire_metadata_tests;
#[path = "../test_support/wire.rs"]
mod wire_test_support;

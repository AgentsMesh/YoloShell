pub const CLAUDE_GENERATION_MUTATION_TYPE: &str = "GenerationMutation";
pub const CLAUDE_GENERATION_MUTATION_ACQUIRE: &str = "acquire";
pub const CLAUDE_NATIVE_STORE_PATH: &str =
    "core/crates/agent/providers/claude/src/generation/native/store.rs";
pub const CLAUDE_NATIVE_PUBLICATION_PATH: &str =
    "core/crates/agent/providers/claude/src/generation/native/store_publication.rs";
pub const CLAUDE_NATIVE_REVOCATION_PATH: &str =
    "core/crates/agent/providers/claude/src/generation/native/revocation.rs";
pub const CLAUDE_NATIVE_APPLY_OWNER: &str = "apply_in_namespace";
pub const CLAUDE_NATIVE_PUBLISH_OWNER: &str = "publish_binding_in_namespace";
pub const CLAUDE_NATIVE_REVOKE_OWNER: &str = "revoke_in_namespace";
pub const CLAUDE_NATIVE_TEST_BOOT_PATH: &str =
    "core/crates/agent/providers/claude/test_support/generation/adversarial_test_support/native.rs";
pub const CLAUDE_NATIVE_TEST_BOOT_OWNER: &str = "persist_native_previous_boot";
pub const CLAUDE_GENERATION_MUTATION_PATH: &str =
    "core/crates/agent/providers/claude/src/generation/mutation.rs";
pub const CLAUDE_GENERATION_STORE_PATH: &str =
    "core/crates/agent/providers/claude/src/generation/store.rs";
pub const CLAUDE_GENERATION_PUBLICATION_PATH: &str =
    "core/crates/agent/providers/claude/src/generation/publication_store.rs";
pub const CLAUDE_GENERATION_DURABLE_PUBLICATION_PATH: &str =
    "core/crates/agent/providers/claude/src/generation/durable_publication.rs";
pub const CLAUDE_GENERATION_UNSTAGED_PUBLICATION_PATH: &str =
    "core/crates/agent/providers/claude/src/generation/durable_publication_unstaged.rs";
pub const CLAUDE_GENERATION_BINDING_IO_PATH: &str =
    "core/crates/agent/providers/claude/src/generation/durable_binding_io.rs";
pub const CLAUDE_GENERATION_FOREGROUND_PATH: &str =
    "core/crates/agent/providers/claude/src/generation/foreground_store.rs";
pub const CLAUDE_GENERATION_RECOVERY_PATH: &str =
    "core/crates/agent/providers/claude/src/generation/recovery_store.rs";
pub const CLAUDE_GENERATION_BINDING_STAGE_PATH: &str =
    "core/crates/agent/providers/claude/src/generation/binding_transition_stage.rs";
pub const CLAUDE_GENERATION_LIFECYCLE_PATH: &str =
    "core/crates/agent/providers/claude/src/generation/lifecycle_store.rs";
pub const CLAUDE_GENERATION_PROCESS_EXIT_PATH: &str =
    "core/crates/agent/providers/claude/src/generation/process_exit.rs";
pub const CLAUDE_GENERATION_BEGIN_OWNER: &str = "begin_foreground_in_namespace";
pub const CLAUDE_GENERATION_EXCHANGE_OWNER: &str = "exchange_set_with_commit_fence";
pub const CLAUDE_GENERATION_EXCHANGE_IN_OWNER: &str = "exchange_set_in";
pub const CLAUDE_GENERATION_FENCE_BINDING_OWNER: &str = "fence_binding_if_active_in_namespace";
pub const CLAUDE_GENERATION_BEGIN_BINDING_OWNER: &str = "begin_binding_publication_in_namespace";
pub const CLAUDE_GENERATION_COMMIT_BINDING_OWNER: &str = "commit_binding_publication_in_namespace";
pub const CLAUDE_GENERATION_PUBLISH_BINDING_OWNER: &str = "publish_binding_state";
pub const CLAUDE_GENERATION_FINISH_PENDING_OWNER: &str = "finish_pending";
pub const CLAUDE_GENERATION_FINISH_UNSTAGED_OWNER: &str = "finish_unstaged";
pub const CLAUDE_GENERATION_WRITE_BINDING_OWNER: &str = "write_binding";
pub const CLAUDE_GENERATION_ACTIVATE_OWNER: &str = "activate_observed_in_namespace";
pub const CLAUDE_GENERATION_APPLY_RECOVERY_OWNER: &str = "apply_recovery_in_namespace";
pub const CLAUDE_GENERATION_APPLY_PARKED_OWNER: &str = "apply_parked_recovery_in_namespace";
pub const CLAUDE_GENERATION_RETIRE_PARKED_OWNER: &str = "retire_parked_in_namespace";
pub const CLAUDE_GENERATION_STAGE_BINDING_OWNER: &str = "stage_binding_transition";
pub const CLAUDE_GENERATION_TRANSITION_OWNER: &str = "transition_in_namespace";
pub const CLAUDE_GENERATION_FINALIZE_EXIT_OWNER: &str = "finalize_process_exit_in_namespace";
pub const CLAUDE_GENERATION_PUBLISH_EXIT_OWNER: &str = "publish_exited";
pub const CLAUDE_GENERATION_TEST_SUPPORT_PATH: &str =
    "core/crates/agent/providers/claude/test_support/generation/adversarial_test_support/state.rs";
pub const CLAUDE_GENERATION_TEST_PREPARE_OWNER: &str = "prepare_durable_state_for_test";
pub const CLAUDE_GENERATION_TEST_PERSIST_OWNER: &str = "persist";
pub const CLAUDE_GENERATION_TEST_PREPARE_STATE_OWNER: &str = "prepare";
pub const CLAUDE_GENERATION_TEST_INSTALL_OWNER: &str = "install_binding";
pub const CLAUDE_GENERATION_TEST_COMMIT_OWNER: &str = "commit";
pub const CLAUDE_GENERATION_STALE_TEST_PATH: &str =
    "core/crates/agent/providers/claude/unit_tests/generation/stale_writer_adversarial_tests.rs";
pub const CLAUDE_GENERATION_STALE_TEST_OWNER: &str =
    "stale_prepared_writer_cannot_overwrite_a_newer_pending_transition";

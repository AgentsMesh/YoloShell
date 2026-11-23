#[cfg(all(test, target_os = "linux", feature = "yolo-test-support"))]
#[path = "../test_support/linux_fixture_containment.rs"]
mod linux_fixture_containment;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/unit/bootstrap/bootstrap_exec_test_hook.rs"]
mod bootstrap_exec_test_hook;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/unit/control/control_test_hooks.rs"]
mod control_test_hooks;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/unit/runtime/runtime_persistence_test_hook.rs"]
mod runtime_persistence_test_hook;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/unit/runtime/runtime_result_test_hook.rs"]
mod runtime_result_test_hook;
#[cfg(feature = "startup-handoff-test-hooks")]
#[doc(hidden)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/unit/startup/startup_handoff_test_hook.rs"]
pub mod startup_handoff_test_hook;
#[cfg(all(test, not(feature = "host-test-hooks")))]
pub(crate) use startup_checkpoint::StartupLaunchFence;
#[cfg(feature = "host-test-hooks")]
pub use startup_checkpoint::StartupLaunchFence;
#[cfg(test)]
pub(crate) use startup_checkpoint::StartupPtyCustodyRollover;
#[cfg(not(feature = "host-test-hooks"))]
pub(crate) use startup_checkpoint::{StartupCheckpointEvidence, source_before_reboot_migration};
#[cfg(feature = "host-test-hooks")]
pub use startup_checkpoint::StartupCheckpointEvidence;
#[cfg(feature = "host-test-hooks")]
pub(crate) use startup_checkpoint::source_before_reboot_migration;
#[cfg(all(test, not(feature = "host-test-hooks")))]
pub(crate) use startup_checkpoint::{
    StartupPhase, StartupProcessIdentity, StartupStage, StartupStatus,
    TEST_WORKER_STARTUP_CHECKPOINT_SCHEMA_VERSION, worker_startup_launch_fence_for_test,
};
#[cfg(feature = "host-test-hooks")]
pub use startup_checkpoint::{
    StartupPhase, StartupProcessIdentity, StartupStage, StartupStatus,
    TEST_WORKER_STARTUP_CHECKPOINT_SCHEMA_VERSION, worker_startup_launch_fence_for_test,
};
pub use startup_checkpoint::{
    WorkerStartupCheckpointPolicy, WorkerStartupCheckpointPreparation,
    WorkerStartupCheckpointReplayDisposition, WorkerStartupCheckpointSettlement,
    prepare_worker_startup_checkpoint_for_launch, settle_worker_startup_checkpoint_for_launch,
    worker_startup_checkpoint_allows_guardian_retirement,
    worker_startup_checkpoint_allows_lifecycle_mutation,
    worker_startup_checkpoint_authorizes_retained_runtime,
    worker_startup_checkpoint_replay_disposition,
};
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/pty/owner/pty_owner_handle_commands_tests.rs"]
mod pty_owner_handle_commands_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../unit_tests/runtime/runtime_result_tests.rs"]
mod runtime_result_tests;
#[cfg(any(test, feature = "termination-test-hooks"))]
#[doc(hidden)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../test_support/unit/termination_test_hook.rs"]
pub mod termination_test_hook;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../unit_tests/termination.rs"]
mod termination_tests;
#[cfg(test)]
#[path = "../test_support/tests.rs"]
mod tests;
mod environment;
mod native_control;
pub use native_control::{
    NativeControlLookupRequest, NativeControlOutcome, NativeControlResult,
    NativeInterruptRequest, NativeObservationRequest, NativeRespondRequest, NativeSubmitRequest,
};
pub use yolo_agent_control_contract::{NativeControlObservation, NativeControlWitness};
pub use yolo_crypto::Sha256Digest as NativeControlDigest;

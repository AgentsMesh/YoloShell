extern crate self as process;
include!("facade_api.rs");

#[cfg(feature = "adversarial-test-hooks")]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/authority_test_hook.rs"]
mod authority_test_hook;
include!("module_wiring.rs");
#[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
pub(crate) use captured_targets::test_hook::install_test_signal_hook;
pub use captured_tree::{
    CapturedProcessTree, FreezePoll, FreezingProcessTree, FrozenProcessTree,
    KillPendingProcessTree, KillPoll, ProcessTransitionError, ReportedProducersGone,
    TerminationUnsupportedProcessTree, TrackedProducersGone,
};
pub use captured_tree::{
    DirectRetirementReady, ExplicitBootstrapFailureRetirementReady, FinalizedProducersGoneReady,
    FreezeReady, FrozenProducersGoneCommit, FrozenProducersGoneFinalization,
    FrozenProducersGonePoll, FrozenProducersGoneReady, KernelEofKillPreparation,
    KernelEofKillReady, KillCommitPoll, KillCommitReady, KillPollReady,
    MasterReleasePendingProducersGone, ProcessClosureVersion, ProducersGoneReportReady,
    ProducersGoneRetirement, ProducersGoneRetirementReady, ReportedProducersGoneRetirement,
    ReportedProducersGoneRetirementReady, SurvivorPoll, SurvivorReady,
};
#[cfg(all(test, target_os = "macos"))]
pub(crate) use captured_tree_macos_pty_fixture::{LineageFixture, PtyLineageFixture};
pub use child_reaper::custody::wait_for_termination_delivery as wait_for_detached_termination_delivery;
pub use child_reaper::{
    DetachedChildHandoffError, ReservedDetachedChildSettlement, handoff_detached_child,
    handoff_detached_child_custody, is_child_already_reaped, reserve_detached_child_settlement,
};
pub use command::configure_detached_command;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
pub use command::spawn_detached_with_control_pipes;
include!("command/command_api_wiring.rs");
#[cfg(any(test, feature = "adversarial-test-hooks"))]
pub use command_diagnostic::spawn_detached_with_diagnostic_pipes;
pub use command_settlement::terminate_detached_child;
#[cfg(not(feature = "runtime-component-tests"))]
pub(crate) use descendant_observation::DescendantObservation;
#[cfg(feature = "runtime-component-tests")]
pub use descendant_observation::DescendantObservation;
pub use detached_child_custody::{DetachedChildCustody, DetachedChildCustodyError};
pub use direct_child_wait::{DirectChildExitStatus, wait as wait_direct_child};
pub use discovery_capability::ProcessDiscoveryCapability;
#[cfg(target_os = "macos")]
pub use discovery_capability::{
    ProcessDiscoveryInheritedWriter, ProcessDiscoveryWriterInheritance,
};
pub use discovery_custody::ProcessDiscoveryCustody;
pub use discovery_custody_budget::process_discovery_force_grace;
pub use exact_process_gone::ExactProcessGone;
#[doc(hidden)]
pub use exact_termination::RetainedExactProcess;
pub use exact_termination::RetainedProcessObservation;
#[rustfmt::skip]
pub use identity::{ProcessIdentityObservation, is_process_alive, observe_process_identity, process_identity_matches, process_start_time, try_process_identity_matches};
pub use identity_relation::{ProcessIdentityRelation, process_identity_relation};
#[cfg(test)]
pub(crate) use identity_relation_test_hook::{
    Guard as ProcessIdentityRelationGuard, install as install_test_process_identity_relation,
};
pub use inventory::ProcessIdentity;
pub use inventory::signal_process_group_member;
pub use inventory::{ProcessGroupIdentity, PtyProcessScope, signal_process_group};
#[cfg(target_os = "linux")]
pub(crate) use linux_passive_pty_master::{LinuxPassivePtyMaster, LinuxPtySlaveIdentity};
#[cfg(target_os = "linux")]
pub use linux_secondary_custody::{
    LINUX_SECONDARY_CUSTODY_DESCRIPTOR_COUNT, LinuxSecondaryCustody,
    LinuxSecondaryCustodyDescriptorParts, LinuxSecondaryCustodyExport,
};
pub use live_identity::LiveProcessIdentity;
#[cfg(target_os = "macos")]
pub use macos_private_descriptor::{
    private_kqueue as macos_private_kqueue,
    private_kqueue_in_spawn_boundary as macos_private_kqueue_in_spawn_boundary,
};
#[cfg(target_os = "macos")]
pub use macos_process_capability::{
    MACOS_PROCESS_CAPABILITY_TRANSFER_MARKER, MacosProcessCapabilityChildWriter,
    MacosProcessCapabilityIdentity, MacosProcessCapabilityIssuer,
    MacosProcessCapabilityManagerReader, MacosProcessCapabilityObservation,
    MacosProcessCapabilityRelease,
};
include!("macos/macos_secondary_custody_api.rs");
pub use monotonic::{monotonic_deadline_ns, monotonic_time_ns};
#[cfg(all(target_os = "macos", any(test, feature = "adversarial-test-hooks")))]
pub use pty_custody_recovery_authority_macos::PtyCustodyRecoveryAuthority;
#[cfg(target_os = "macos")]
pub use pty_master_fd_custody_probe_macos::PtyMasterFdCustodyProbe;
pub use pty_master_release::{
    ProcessClosureIdentity, PtyHostReleaseVerifier, PtyHostReleasedAck,
    PtyLocalMasterAliasesClosed, PtyMasterAliasClosureSettlement, PtyMasterAliasReleaseAuthority,
    PtyMasterAliasesClosed, PtyRuntimeIdentity,
};
pub use pty_session_proof::CapabilityScanActivity;
#[cfg(target_os = "macos")]
pub(crate) use pty_session_proof::MacosPassivePtyMaster;
pub use pty_session_proof::PtySessionProof;
pub use pty_session_proof::{
    BootstrapExecTransitionAuthority, CommittedPtyReopenSealAuthority,
    ExplicitBootstrapExecFailureProof,
};
include!("resilience/resilience_api_wiring.rs");
#[cfg(all(test, unix, feature = "yolo-integration-tests"))]
pub(crate) use sigchld_waitability::reject_spawn_gate_entry_for_test;
#[cfg(unix)]
pub use sigchld_waitability::{
    WaitableChildSpawnBoundary, try_waitable_child_spawn_guard, waitable_child_spawn_guard_before,
};
#[cfg(unix)]
pub use sigchld_waitability::{WaitableChildSpawnGuard, waitable_child_spawn_guard};
pub use signal_authority::DestructiveSessionUnavailable;
pub use unidentified_pty_child_custody::{UnidentifiedPtyChildCustody, retain_unidentified_child};
#[rustfmt::skip]
pub use unreaped_root_exit::{DirectChildIdentity, LostRootWaitAuthority, OwnedPtyChild, RootGoneWithoutWait, RootWaitAuthority, RootWaitBinding, RootWaitReapError, UnreapedRootExit, WaitAuthorityLoss, is_wait_authority_loss};
include!("test_wiring.rs");
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/tests.rs"]
mod process_integration_tests;

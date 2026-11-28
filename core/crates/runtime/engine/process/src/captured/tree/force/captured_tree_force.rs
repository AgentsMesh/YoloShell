use super::{CapturedProcessTree, freeze};
use crate::process::LiveProcessIdentity;
#[path = "captured_tree_force_commit.rs"]
mod commit;
#[path = "captured_tree_force_empty.rs"]
mod empty;
#[path = "captured_tree_force_empty_commit_state.rs"]
mod empty_commit_state;
#[path = "captured_tree_force_empty_finalize.rs"]
mod empty_finalize;
#[path = "captured_tree_force_empty_transaction.rs"]
mod empty_transaction;
#[path = "captured_tree_force_freeze.rs"]
mod freeze_state;
#[path = "captured_tree_force_kernel_eof.rs"]
mod kernel_eof;
#[path = "captured_tree_force_kill_commit.rs"]
mod kill_commit;
#[path = "captured_tree_force_kill_poll.rs"]
mod kill_poll;
#[path = "../captured_tree_post_kill.rs"]
mod post_kill;
#[path = "captured_tree_force_reopen_seal_cleanup.rs"]
mod reopen_seal_cleanup;
#[path = "captured_tree_force_reopen_seal_terminal.rs"]
mod reopen_seal_terminal;
#[path = "captured_tree_force_reopen_seal_transaction.rs"]
mod reopen_seal_transaction;
#[path = "captured_tree_force_reopen_seal_types.rs"]
mod reopen_seal_types;
#[path = "captured_tree_force_root_exit.rs"]
mod root_exit;
#[path = "captured_tree_force_version.rs"]
mod version;

#[cfg_attr(any(test, feature = "runtime-component-tests"), derive(Debug))]
enum FreezeConfirmation {
    Unconfirmed,
    Observed(freeze::FreezeClosureCandidate),
}

enum GoneConfirmation {
    Unconfirmed,
    ObservedEmpty,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum FrozenKillBasis {
    ExactPositive,
    IncrementalExactPositive,
    Ordinary,
    KernelEof,
}

pub struct GonePendingReady(GoneConfirmation);

pub struct KillGoneReady(KillGoneAuthority);

pub struct FrozenProducersGoneReady {
    version: ProcessClosureVersion,
    authority: FrozenGoneAuthority,
    transaction: empty_transaction::ProducersGoneTransaction,
    kernel_eof: Option<crate::process::kernel_eof_authority::KernelEofCommitBinding>,
    #[cfg(target_os = "linux")]
    retained: Option<crate::process::linux_containment::LinuxContainmentCommitReady>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum FrozenGoneAuthority {
    GlobalExclusion,
    KernelEof,
}

pub use reopen_seal_types::{FrozenProducersGoneCommit, KernelEofReopenSealCommit};

pub enum FrozenProducersGonePoll {
    NotEmpty,
    Pending,
    Refreeze(KillCommitReady),
    Gone(FrozenProducersGoneReady),
}

pub struct FreezingProcessTree {
    pub(super) tree: CapturedProcessTree,
    confirmation: FreezeConfirmation,
    previous_version: ProcessClosureVersion,
    pub(super) destructive_started: bool,
}

pub struct FrozenProcessTree {
    pub(super) tree: CapturedProcessTree,
    closure: Vec<LiveProcessIdentity>,
    version: ProcessClosureVersion,
    kill_basis: FrozenKillBasis,
    kernel_eof: Option<crate::process::kernel_eof_authority::KernelEofCommitBinding>,
    destructive_started: bool,
    producers_gone_authority: Option<FrozenGoneAuthority>,
    producers_gone_transaction: Option<empty_transaction::ProducersGoneTransactionState>,
    refreeze_required: bool,
}

pub struct KillPendingProcessTree {
    pub(super) tree: CapturedProcessTree,
    confirmation: GoneConfirmation,
    post_kill: post_kill::PostKillExactClosure,
    version: ProcessClosureVersion,
    destructive_started: bool,
}

#[derive(Clone, Debug)]
pub struct ProcessClosureVersion {
    identity: crate::process::ProcessClosureIdentity,
    generation: u64,
}

impl PartialEq for ProcessClosureVersion {
    fn eq(&self, other: &Self) -> bool {
        self.generation == other.generation && self.identity.same_allocation(&other.identity)
    }
}

impl Eq for ProcessClosureVersion {}

pub enum FreezeReady {
    Pending(FreezePendingReady),
    KernelEof(KernelEofFreezeReady),
    Frozen(FrozenFreezeReady),
}

include!("captured_tree_force_ready_types.rs");

pub enum KillCommitReady {
    Refreeze,
    PendingExact(super::termination::ExactClosureKill),
    PendingIncremental(super::termination::ExactClosureKill),
    PendingOrdinary(OrdinaryKillReady),
    PendingKernelEof(KernelEofKillCommitReady),
}

pub struct OrdinaryKillReady {
    version: ProcessClosureVersion,
}

pub struct KernelEofKillCommitReady {
    version: ProcessClosureVersion,
}

pub enum KernelEofKillPreparation {
    Refreeze,
    FreshEofRequired,
    Ready(KernelEofKillReady),
}

pub struct KernelEofKillReady {
    version: ProcessClosureVersion,
    seal: crate::process::CommittedPtyReopenSealAuthority,
}

pub enum KillPollReady {
    Refreeze,
    Pending(GonePendingReady),
    Gone(KillGoneReady),
}

#[derive(Clone, Copy)]
pub(crate) enum KillGoneAuthority {
    ExactContainment,
    GlobalExclusion,
}

pub enum FreezePoll {
    Pending(FreezingProcessTree),
    Frozen(FrozenProcessTree),
}

pub enum KillPoll {
    Refreeze(FreezingProcessTree),
    Pending(KillPendingProcessTree),
    Gone(super::gone::TrackedProducersGone),
}

pub enum KillCommitPoll {
    Refreeze(FreezingProcessTree),
    Pending(KillPendingProcessTree),
}

pub use empty_transaction::{FinalizedProducersGoneReady, FrozenProducersGoneFinalization};

#[cfg(any(all(test, feature = "yolo-integration-tests"), feature = "runtime-component-tests"))]
include!("../../../../test_support/captured_tree/captured_tree_force.rs");
#[cfg(all(test, feature = "yolo-integration-tests"))]
include!("../../../../integration_tests/captured/tree/force/captured_tree_force_test_wiring.rs");

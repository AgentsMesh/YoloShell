use super::live_identity::LiveProcessIdentityKey;
use super::{CapturedProcessTree, DirectChildExitStatus, LiveProcessIdentity, ProcessIdentity};
use anyhow::{Context, Result, ensure};
use portable_pty::Child;
use std::io;
use std::mem::MaybeUninit;

#[path = "unreaped_root_exit_containment.rs"]
mod containment;
#[path = "unreaped_root_exit_loss.rs"]
mod loss;
#[path = "unreaped_root_exit_observe.rs"]
mod observe;
#[path = "unreaped_root_exit_signal.rs"]
mod signal;
#[path = "unreaped_root_exit_wait.rs"]
mod wait;
use containment::child_process_id;
pub use loss::{LostRootWaitAuthority, RootGoneWithoutWait, RootWaitReapError, WaitAuthorityLoss};

const WAITID_OPTIONS: i32 = libc::WEXITED | libc::WNOWAIT | libc::WNOHANG;
const WAITID_FAILURE_RESULT: libc::c_int = -1;
const NO_EXIT_PID: libc::pid_t = 0;
const ROOT_PID_RANGE_CONTEXT: &str = "PTY root exceeds waitid range";
const WAITID_CONTEXT: &str = "observe PTY root exit without reaping";
const UNEXPECTED_CHILD_DETAIL: &str = "waitid observed an unexpected PTY child";
const CHILD_PID_QUERY_PANIC_DETAIL: &str = "owned PTY child process-id query panicked";
const CHILD_PID_UNAVAILABLE_DETAIL: &str = "owned PTY child did not expose its process id";
const CHILD_PID_MISMATCH_DETAIL: &str = "owned PTY child does not match captured process tree";
const CHILD_IDENTITY_CAPTURE_CONTEXT: &str = "capture waitable PTY child identity";
const CHILD_LIVE_IDENTITY_DETAIL: &str = "waitable PTY child identity is no longer live";
const CHILD_IDENTITY_CHANGED_DETAIL: &str = "waitable PTY child identity changed during binding";
const ROOT_EXIT_PROOF_MISMATCH_DETAIL: &str = "PTY root reap proof does not match wait authority";

pub type OwnedPtyChild = Box<dyn Child + Send + Sync>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectChildIdentity {
    root: ProcessIdentity,
    live_root: LiveProcessIdentity,
}

#[cfg(not(target_os = "linux"))]
impl Copy for DirectChildIdentity {}

pub struct RootWaitBinding {
    root: ProcessIdentity,
    live_root: LiveProcessIdentity,
}

pub struct RootWaitAuthority {
    child: OwnedPtyChild,
    root: ProcessIdentity,
    live_root: LiveProcessIdentity,
    waited_status: Option<DirectChildExitStatus>,
}

#[derive(Clone, Copy, Debug)]
pub struct UnreapedRootExit {
    root: ProcessIdentity,
    live_root: LiveProcessIdentityKey,
}

pub fn is_wait_authority_loss(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<io::Error>()
            .is_some_and(|error| error.raw_os_error() == Some(libc::ECHILD))
    })
}

impl RootWaitAuthority {
    pub fn prove_owned_child(
        child: &OwnedPtyChild,
        tree: &CapturedProcessTree,
    ) -> Result<RootWaitBinding> {
        DirectChildIdentity {
            root: tree.root(),
            live_root: tree.live_root(),
        }
        .prove_current(child)?;
        Ok(RootWaitBinding {
            root: tree.root(),
            live_root: tree.live_root(),
        })
    }

    pub fn root(&self) -> ProcessIdentity {
        self.root
    }

    pub fn into_untracked_child(self) -> OwnedPtyChild {
        self.child
    }

    pub fn reap(self, proof: UnreapedRootExit) -> Result<DirectChildExitStatus> {
        self.reap_preserving_authority(proof)
            .map_err(RootWaitReapError::into_retained_error)
    }
}

impl DirectChildIdentity {
    pub fn capture_candidate(child: &OwnedPtyChild) -> Result<Self> {
        let process_id = child_process_id(child)?;
        prove_waitable(process_id)?;
        let root = ProcessIdentity::capture(process_id).context(CHILD_IDENTITY_CAPTURE_CONTEXT)?;
        let live_root = LiveProcessIdentity::capture(root)?.context(CHILD_LIVE_IDENTITY_DETAIL)?;
        Ok(Self { root, live_root })
    }

    pub fn prove_current(&self, child: &OwnedPtyChild) -> Result<()> {
        let process_id = child_process_id(child)?;
        ensure!(process_id == self.root.pid, CHILD_PID_MISMATCH_DETAIL);
        prove_waitable(process_id)?;
        ensure!(self.live_root.try_matches()?, CHILD_IDENTITY_CHANGED_DETAIL);
        prove_waitable(process_id)?;
        ensure!(self.live_root.try_matches()?, CHILD_IDENTITY_CHANGED_DETAIL);
        Ok(())
    }

    pub fn root(&self) -> ProcessIdentity {
        self.root
    }

    pub fn live_root(&self) -> LiveProcessIdentity {
        self.live_root.clone()
    }
}

impl RootWaitBinding {
    pub fn bind(self, child: OwnedPtyChild) -> RootWaitAuthority {
        RootWaitAuthority {
            child,
            root: self.root,
            live_root: self.live_root,
            waited_status: None,
        }
    }
}

impl UnreapedRootExit {
    pub(super) fn identities(&self) -> (ProcessIdentity, LiveProcessIdentityKey) {
        (self.root, self.live_root)
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn from_exact_exit_event(
        root: ProcessIdentity,
        live_root: LiveProcessIdentity,
    ) -> Self {
        Self {
            root,
            live_root: live_root.key(),
        }
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub(super) fn for_test(root: ProcessIdentity, live_root: LiveProcessIdentity) -> Self {
        Self {
            root,
            live_root: live_root.key(),
        }
    }
}

fn prove_waitable(process_id: u32) -> Result<()> {
    let pid = libc::id_t::try_from(process_id).context(ROOT_PID_RANGE_CONTEXT)?;
    loop {
        let mut information = MaybeUninit::<libc::siginfo_t>::zeroed();
        let result =
            unsafe { libc::waitid(libc::P_PID, pid, information.as_mut_ptr(), WAITID_OPTIONS) };
        if result != WAITID_FAILURE_RESULT {
            let observed = unsafe { information.assume_init().si_pid() };
            ensure!(
                observed == NO_EXIT_PID || observed == process_id as libc::pid_t,
                "{UNEXPECTED_CHILD_DETAIL} {observed}"
            );
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error).context(WAITID_CONTEXT);
        }
    }
}

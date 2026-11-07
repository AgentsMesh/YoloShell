use anyhow::{Result, ensure};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

const AUTHORITY_ALREADY_CLAIMED_DETAIL: &str =
    "bootstrap exec-transition authority was already claimed";
const TRANSITION_ALREADY_SETTLED_DETAIL: &str = "bootstrap exec transition was already settled";
const FAILURE_PROOF_MISMATCH_DETAIL: &str =
    "explicit bootstrap exec failure belongs to another PTY proof";

#[derive(Clone, Default)]
pub(super) struct BootstrapExecState(Option<Arc<SharedState>>);

pub struct BootstrapExecTransitionAuthority {
    shared: Arc<SharedState>,
    settled: bool,
}

pub struct ExplicitBootstrapExecFailureProof {
    shared: Arc<SharedState>,
}

struct SharedState {
    authority_claimed: AtomicBool,
    transition: AtomicU8,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Transition {
    Armed,
    ExecConfirmed,
    ExplicitFailure,
    Indeterminate,
    FailureConsumed,
}

impl BootstrapExecState {
    pub(super) fn armed() -> Self {
        Self(Some(Arc::new(SharedState {
            authority_claimed: AtomicBool::new(false),
            transition: AtomicU8::new(Transition::Armed as u8),
        })))
    }

    pub(super) fn take_authority(&self) -> Result<BootstrapExecTransitionAuthority> {
        let shared = self.shared()?;
        ensure!(
            shared
                .authority_claimed
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok(),
            AUTHORITY_ALREADY_CLAIMED_DETAIL
        );
        Ok(BootstrapExecTransitionAuthority {
            shared: Arc::clone(shared),
            settled: false,
        })
    }

    pub(super) fn consume_failure(&self, proof: ExplicitBootstrapExecFailureProof) -> Result<()> {
        let shared = self.shared()?;
        ensure!(
            Arc::ptr_eq(shared, &proof.shared),
            FAILURE_PROOF_MISMATCH_DETAIL
        );
        transition(
            shared,
            Transition::ExplicitFailure,
            Transition::FailureConsumed,
        )
    }

    pub(super) fn is_confirmed(&self) -> bool {
        self.0
            .as_ref()
            .is_some_and(|shared| shared.load() == Transition::ExecConfirmed)
    }

    fn shared(&self) -> Result<&Arc<SharedState>> {
        self.0
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!(FAILURE_PROOF_MISMATCH_DETAIL))
    }
}

impl BootstrapExecTransitionAuthority {
    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn for_test() -> Self {
        BootstrapExecState::armed().take_authority().unwrap()
    }

    pub fn confirm_exec(mut self) -> Result<()> {
        transition(&self.shared, Transition::Armed, Transition::ExecConfirmed)?;
        self.settled = true;
        Ok(())
    }

    pub fn confirm_explicit_failure(mut self) -> Result<ExplicitBootstrapExecFailureProof> {
        transition(&self.shared, Transition::Armed, Transition::ExplicitFailure)?;
        self.settled = true;
        Ok(ExplicitBootstrapExecFailureProof {
            shared: Arc::clone(&self.shared),
        })
    }

    pub fn cancel_before_release(self) -> Result<ExplicitBootstrapExecFailureProof> {
        self.confirm_explicit_failure()
    }
}

impl Drop for BootstrapExecTransitionAuthority {
    fn drop(&mut self) {
        if !self.settled {
            let _ = self.shared.transition.compare_exchange(
                Transition::Armed as u8,
                Transition::Indeterminate as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
    }
}

impl SharedState {
    fn load(&self) -> Transition {
        match self.transition.load(Ordering::Acquire) {
            value if value == Transition::Armed as u8 => Transition::Armed,
            value if value == Transition::ExecConfirmed as u8 => Transition::ExecConfirmed,
            value if value == Transition::ExplicitFailure as u8 => Transition::ExplicitFailure,
            value if value == Transition::Indeterminate as u8 => Transition::Indeterminate,
            _ => Transition::FailureConsumed,
        }
    }
}

fn transition(shared: &SharedState, from: Transition, to: Transition) -> Result<()> {
    ensure!(
        shared
            .transition
            .compare_exchange(from as u8, to as u8, Ordering::AcqRel, Ordering::Acquire,)
            .is_ok(),
        TRANSITION_ALREADY_SETTLED_DETAIL
    );
    Ok(())
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../../../unit_tests/pty_session_bootstrap_exec_inline_tests.rs"]
mod tests;

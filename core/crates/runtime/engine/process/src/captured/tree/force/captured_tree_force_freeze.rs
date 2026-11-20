use super::{
    CapturedProcessTree, FreezeConfirmation, FreezePendingReady, FreezePoll, FreezeReady,
    FreezingProcessTree, FrozenFreezeReady, FrozenKillBasis, FrozenProcessTree,
    KernelEofFreezeReady, ProcessClosureVersion,
};
use crate::process::captured_tree::freeze;
use crate::process::captured_tree::transition::ProcessTransitionError;
use anyhow::Result;

impl CapturedProcessTree {
    pub fn begin_force(self) -> FreezingProcessTree {
        let previous_version = self.closure_version.clone();
        FreezingProcessTree::enter(self, previous_version, false)
    }
}

impl FreezingProcessTree {
    pub fn requires_exact_root_force_fallback(&self) -> bool {
        !self.destructive_started && self.tree.require_force_transaction_start().is_err()
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn confirmation_for_test(&self) -> String {
        format!("{:?}", self.confirmation)
    }

    pub(in crate::process::captured_tree) fn enter(
        mut tree: CapturedProcessTree,
        previous_version: ProcessClosureVersion,
        destructive_started: bool,
    ) -> Self {
        tree.begin_frozen_discovery();
        Self {
            tree,
            confirmation: FreezeConfirmation::Unconfirmed,
            previous_version,
            destructive_started,
        }
    }

    pub fn poll(mut self) -> Result<FreezePoll, ProcessTransitionError<Self>> {
        let ready = match self.prepare_poll() {
            Ok(ready) => ready,
            Err(error) => return Err(ProcessTransitionError::new(self, error)),
        };
        Ok(self.commit_poll(ready))
    }

    pub fn prepare_poll(&mut self) -> Result<FreezeReady> {
        let previous = std::mem::replace(&mut self.confirmation, FreezeConfirmation::Unconfirmed);
        let Some(candidate) = freeze::observe(&mut self.tree, &mut self.destructive_started)?
        else {
            return Ok(FreezeReady::Pending(FreezePendingReady(None)));
        };
        if !matches!(previous, FreezeConfirmation::Observed(ref observed) if observed == &candidate)
        {
            return Ok(FreezeReady::Pending(FreezePendingReady(Some(candidate))));
        }
        let version = self.previous_version.next()?;
        Ok(match candidate {
            freeze::FreezeClosureCandidate::Exact(closure) => {
                FreezeReady::Frozen(FrozenFreezeReady {
                    closure,
                    version,
                    kill_basis: FrozenKillBasis::ExactPositive,
                })
            }
            freeze::FreezeClosureCandidate::IncrementalExact(closure) => {
                FreezeReady::Frozen(FrozenFreezeReady {
                    closure,
                    version,
                    kill_basis: FrozenKillBasis::IncrementalExactPositive,
                })
            }
            freeze::FreezeClosureCandidate::Ordinary(closure) => {
                FreezeReady::Frozen(FrozenFreezeReady {
                    closure,
                    version,
                    kill_basis: FrozenKillBasis::Ordinary,
                })
            }
            freeze::FreezeClosureCandidate::KernelEof(closure) => {
                FreezeReady::KernelEof(KernelEofFreezeReady {
                    closure,
                    version,
                    authority: None,
                })
            }
        })
    }

    pub fn commit_poll(mut self, ready: FreezeReady) -> FreezePoll {
        match ready {
            FreezeReady::Pending(FreezePendingReady(Some(candidate))) => self.pending(candidate),
            FreezeReady::Pending(FreezePendingReady(None)) => {
                self.confirmation = FreezeConfirmation::Unconfirmed;
                FreezePoll::Pending(self)
            }
            FreezeReady::KernelEof(mut ready) => match ready.authority.take() {
                Some(authority) => {
                    self.tree.closure_version = ready.version.clone();
                    FreezePoll::Frozen(FrozenProcessTree {
                        tree: self.tree,
                        closure: ready.closure,
                        version: ready.version,
                        kill_basis: FrozenKillBasis::KernelEof,
                        kernel_eof: Some(authority),
                        destructive_started: self.destructive_started,
                        producers_gone_authority: None,
                        producers_gone_transaction: None,
                        refreeze_required: false,
                    })
                }
                None => self.pending(freeze::FreezeClosureCandidate::KernelEof(ready.closure)),
            },
            FreezeReady::Frozen(FrozenFreezeReady {
                closure,
                version,
                kill_basis,
            }) => {
                self.tree.closure_version = version.clone();
                FreezePoll::Frozen(FrozenProcessTree {
                    tree: self.tree,
                    closure,
                    version,
                    kill_basis,
                    kernel_eof: None,
                    destructive_started: self.destructive_started,
                    producers_gone_authority: None,
                    producers_gone_transaction: None,
                    refreeze_required: false,
                })
            }
        }
    }

    fn pending(mut self, candidate: freeze::FreezeClosureCandidate) -> FreezePoll {
        self.confirmation = FreezeConfirmation::Observed(candidate);
        FreezePoll::Pending(self)
    }
}

impl FreezeReady {
    pub fn kernel_eof_version(&self) -> Option<&ProcessClosureVersion> {
        match self {
            Self::KernelEof(ready) => Some(&ready.version),
            Self::Pending(_) | Self::Frozen(_) => None,
        }
    }

    pub fn authorize_kernel_eof(
        self,
        permit: crate::process::kernel_eof_authority::KernelEofPermit,
    ) -> Self {
        match self {
            Self::KernelEof(mut ready) => {
                ready.authority = permit.authorize(&ready.version);
                Self::KernelEof(ready)
            }
            ready => ready,
        }
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn authorize_kernel_eof_for_test(self) -> Self {
        let Some(version) = self.kernel_eof_version().cloned() else {
            return self;
        };
        let output = crate::process::kernel_eof_authority::KernelPtyEofIdentity::from_kernel_read();
        let (permit, _) =
            crate::process::kernel_eof_authority::bind_scripted_kernel_eof(&version, &output);
        self.authorize_kernel_eof(permit)
    }
}

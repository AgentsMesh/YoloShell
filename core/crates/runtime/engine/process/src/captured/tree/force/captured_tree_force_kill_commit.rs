use super::{
    FreezingProcessTree, FrozenKillBasis, FrozenProcessTree, GoneConfirmation, KillCommitPoll,
    KillCommitReady, KillPendingProcessTree,
};
impl FrozenProcessTree {
    pub fn commit_kill(self, ready: KillCommitReady) -> KillCommitPoll {
        match ready {
            KillCommitReady::Refreeze => self.refreeze(),
            KillCommitReady::PendingExact(exact) => {
                self.commit_exact(exact, FrozenKillBasis::ExactPositive)
            }
            KillCommitReady::PendingIncremental(exact) => {
                self.commit_exact(exact, FrozenKillBasis::IncrementalExactPositive)
            }
            KillCommitReady::PendingOrdinary(ready) => {
                if self.kill_basis != FrozenKillBasis::Ordinary || ready.version != self.version {
                    return self.refreeze();
                }
                self.pending(super::post_kill::PostKillExactClosure::ordinary())
            }
            KillCommitReady::PendingKernelEof(ready) => {
                if self.kill_basis != FrozenKillBasis::KernelEof || ready.version != self.version {
                    return self.refreeze();
                }
                self.pending(super::post_kill::PostKillExactClosure::ordinary())
            }
        }
    }

    fn commit_exact(
        mut self,
        exact_kill: super::super::ExactClosureKill,
        basis: FrozenKillBasis,
    ) -> KillCommitPoll {
        let authority = match basis {
            FrozenKillBasis::ExactPositive => {
                super::super::termination::ExactKillAuthority::ExactPositive
            }
            FrozenKillBasis::IncrementalExactPositive => {
                super::super::termination::ExactKillAuthority::IncrementalExactPositive
            }
            FrozenKillBasis::Ordinary | FrozenKillBasis::KernelEof => unreachable!(),
        };
        if self.kill_basis != basis || !exact_kill.matches(&self.closure, &self.version, authority)
        {
            return self.refreeze();
        }
        self.tree.begin_frozen_discovery();
        self.pending(super::post_kill::PostKillExactClosure::exact(exact_kill))
    }

    fn pending(self, post_kill: super::post_kill::PostKillExactClosure) -> KillCommitPoll {
        KillCommitPoll::Pending(KillPendingProcessTree {
            tree: self.tree,
            confirmation: GoneConfirmation::Unconfirmed,
            post_kill,
            version: self.version,
            destructive_started: self.destructive_started,
        })
    }

    fn refreeze(self) -> KillCommitPoll {
        KillCommitPoll::Refreeze(FreezingProcessTree::enter(
            self.tree,
            self.version,
            self.destructive_started,
        ))
    }
}

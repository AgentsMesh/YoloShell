use super::{CapturedProcessTree, ProcessIdentity};
use anyhow::Result;
const IDENTITY_INDETERMINATE: &str = "exact process identity is indeterminate";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetainedProcessObservation {
    Running,
    Settled,
    Indeterminate,
}

pub struct RetainedExactProcess {
    authority: RetainedExactAuthority,
}
enum RetainedExactAuthority {
    Destructive(CapturedProcessTree),
    ExactObserver(CapturedProcessTree),
}

impl RetainedExactProcess {
    pub fn capture(identity: ProcessIdentity) -> Result<Option<Self>> {
        Self::capture_with(identity, CapturedProcessTree::capture_lineage)
            .map(|tree| tree.map(Self::destructive))
    }

    pub(crate) fn capture_pty(
        identity: ProcessIdentity,
        proof: super::PtySessionProof,
    ) -> Result<Option<Self>> {
        if proof.requires_pty_object_lease() && !proof.has_required_pty_object_lease() {
            return Self::capture_with(identity, CapturedProcessTree::capture_lineage)
                .map(|tree| tree.map(Self::exact_observer));
        }
        Self::capture_with(identity, |identity| {
            CapturedProcessTree::capture_pty(identity, proof)
        })
        .map(|tree| tree.map(Self::destructive))
    }

    #[cfg(feature = "adversarial-test-hooks")]
    pub fn capture_pty_for_test(
        identity: ProcessIdentity,
        proof: super::PtySessionProof,
    ) -> Result<Option<Self>> {
        Self::capture_pty(identity, proof)
    }

    pub fn capture_pty_excluding(
        identity: ProcessIdentity,
        proof: super::PtySessionProof,
        excluded: impl IntoIterator<Item = ProcessIdentity>,
    ) -> Result<Option<Self>> {
        let excluded = excluded.into_iter().collect::<Vec<_>>();
        if proof.requires_pty_object_lease() && !proof.has_required_pty_object_lease() {
            return Self::capture_with(identity, |identity| {
                CapturedProcessTree::capture_lineage_excluding(identity, excluded)
            })
            .map(|tree| tree.map(Self::exact_observer));
        }
        Self::capture_with(identity, |identity| {
            CapturedProcessTree::capture_pty_excluding(identity, proof, excluded)
        })
        .map(|tree| tree.map(Self::destructive))
    }

    fn capture_with(
        identity: ProcessIdentity,
        capture: impl FnOnce(ProcessIdentity) -> Result<CapturedProcessTree>,
    ) -> Result<Option<CapturedProcessTree>> {
        match super::process_identity_relation(identity.pid, identity.start_time) {
            super::ProcessIdentityRelation::Gone => return Ok(None),
            super::ProcessIdentityRelation::Unknown => anyhow::bail!(IDENTITY_INDETERMINATE),
            super::ProcessIdentityRelation::Matching => {}
        }
        match capture(identity) {
            Ok(tree) => Ok(Some(tree)),
            Err(error) => match super::process_identity_relation(identity.pid, identity.start_time)
            {
                super::ProcessIdentityRelation::Gone => Ok(None),
                super::ProcessIdentityRelation::Matching
                | super::ProcessIdentityRelation::Unknown => Err(error),
            },
        }
    }

    pub fn observe(&mut self) -> Result<RetainedProcessObservation> {
        match &mut self.authority {
            RetainedExactAuthority::Destructive(tree) => observe_destructive(tree),
            RetainedExactAuthority::ExactObserver(tree) => observe_exact(tree),
        }
    }

    pub fn is_running(&mut self) -> Result<bool> {
        Ok(self.observe()? != RetainedProcessObservation::Settled)
    }

    pub fn begin_containment(&mut self) -> Result<bool> {
        match &mut self.authority {
            RetainedExactAuthority::Destructive(tree) => {
                tree.reconcile_retained_root_exit()?;
                Ok(!tree.request_graceful_termination()?.is_empty())
            }
            RetainedExactAuthority::ExactObserver(_) => Ok(false),
        }
    }

    pub fn enforce_containment(&mut self) -> Result<bool> {
        match &mut self.authority {
            RetainedExactAuthority::Destructive(tree) => {
                tree.reconcile_retained_root_exit()?;
                Ok(!tree.request_force_termination()?.is_empty())
            }
            RetainedExactAuthority::ExactObserver(_) => Ok(false),
        }
    }

    fn destructive(tree: CapturedProcessTree) -> Self {
        Self {
            authority: RetainedExactAuthority::Destructive(tree),
        }
    }

    fn exact_observer(tree: CapturedProcessTree) -> Self {
        Self {
            authority: RetainedExactAuthority::ExactObserver(tree),
        }
    }

    fn tree(&self) -> &CapturedProcessTree {
        match &self.authority {
            RetainedExactAuthority::Destructive(tree)
            | RetainedExactAuthority::ExactObserver(tree) => tree,
        }
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    fn tree_mut(&mut self) -> &mut CapturedProcessTree {
        match &mut self.authority {
            RetainedExactAuthority::Destructive(tree)
            | RetainedExactAuthority::ExactObserver(tree) => tree,
        }
    }
}

#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[path = "../../test_support/exact_termination.rs"]
mod test_support;

fn observe_destructive(tree: &mut CapturedProcessTree) -> Result<RetainedProcessObservation> {
    tree.reconcile_retained_root_exit()?;
    if !tree.observe_live_identities()?.is_empty() {
        return Ok(RetainedProcessObservation::Running);
    }
    tree.reconcile_retained_exact_empty()?;
    Ok(match tree.descendant_observation() {
        super::DescendantObservation::Settled => RetainedProcessObservation::Settled,
        _ => RetainedProcessObservation::Indeterminate,
    })
}

fn observe_exact(tree: &CapturedProcessTree) -> Result<RetainedProcessObservation> {
    if !tree.live_identities()?.is_empty() {
        return Ok(RetainedProcessObservation::Running);
    }
    let root = tree.root();
    Ok(
        match super::process_identity_relation(root.pid, root.start_time) {
            super::ProcessIdentityRelation::Matching => RetainedProcessObservation::Running,
            _ => RetainedProcessObservation::Indeterminate,
        },
    )
}

#[cfg(any(test, feature = "adversarial-test-hooks"))]
impl ProcessIdentity {
    #[doc(hidden)]
    pub fn enforce_exact_test_cleanup(self) -> Result<()> {
        let Some(tree) =
            RetainedExactProcess::capture_with(self, CapturedProcessTree::capture_lineage)?
        else {
            return Ok(());
        };
        let mut retained = RetainedExactProcess::destructive(tree);
        retained.enforce_containment()?;
        Ok(())
    }
}

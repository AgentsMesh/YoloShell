use super::super::startup_scope::{StartupScopeMonitor, new_tracked_tree};
use super::StartupOutputProgress;
use super::{CHILD_IDENTITY_MISMATCH_DETAIL, STARTUP_CHILD_UNAVAILABLE_CONTEXT, StartupChildGuard};
use crate::process::{
    CapturedProcessTree, DirectChildIdentity, ProcessIdentity, PtySessionProof, RootWaitAuthority,
    WaitableChildSpawnBoundary,
};
use anyhow::{Context, Result, ensure};
use std::sync::Arc;

pub(crate) const BOOTSTRAP_FALLBACK_UNAVAILABLE_CONTEXT: &str =
    "startup bootstrap fallback custody is unavailable";

impl StartupChildGuard {
    #[cfg(target_os = "macos")]
    pub(crate) fn install_process_capability(
        &mut self,
        authority: std::sync::Arc<crate::process::MacosProcessCapabilityManagerReader>,
    ) -> Result<()> {
        self.tree
            .as_ref()
            .context(super::STARTUP_TREE_UNAVAILABLE_CONTEXT)?
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .install_process_capability(authority)
    }

    pub(crate) fn bind_pty(
        &mut self,
        mut proof: PtySessionProof,
        boundary: &WaitableChildSpawnBoundary<'_>,
    ) -> Result<ProcessIdentity> {
        let direct = match self.capture_direct_identity() {
            Ok(direct) => direct,
            Err(error) => {
                if self.identity.is_none() && self.live_identity.is_none() {
                    self.settle_spawn_without_identity(boundary);
                }
                return Err(error);
            }
        };
        self.remember_direct_identity(&direct);
        let root = direct.root();
        let master_alias_authority = proof.bind_master_alias_release(root)?;
        let Some(StartupOutputProgress::Exclusive(output)) = self.output_progress.as_mut() else {
            anyhow::bail!(super::STARTUP_OUTPUT_PROGRESS_UNAVAILABLE_CONTEXT);
        };
        output.bind_master_alias_authority(master_alias_authority)?;
        let tree = CapturedProcessTree::capture_pty_in_spawn_boundary(root, proof, boundary)?;
        self.bind_identified_tree(direct, tree)?;
        Ok(root)
    }

    #[cfg(test)]
    pub(crate) fn track(&mut self, identity: ProcessIdentity) -> Result<()> {
        let tree = CapturedProcessTree::capture(identity)?;
        let direct = self.capture_direct_identity()?;
        ensure!(direct.root() == identity, CHILD_IDENTITY_MISMATCH_DETAIL);
        self.bind_identified_tree(direct, tree)?;
        self.commit_bootstrap_gate_release()
    }

    #[cfg(test)]
    pub(crate) fn track_pty(
        &mut self,
        identity: ProcessIdentity,
        proof: PtySessionProof,
    ) -> Result<()> {
        let tree = CapturedProcessTree::capture_pty_for_test(identity, proof)?;
        let direct = self.capture_direct_identity()?;
        ensure!(direct.root() == identity, CHILD_IDENTITY_MISMATCH_DETAIL);
        self.bind_identified_tree(direct, tree)?;
        self.commit_bootstrap_gate_release()
    }

    pub(crate) fn capture_direct_identity(&self) -> Result<DirectChildIdentity> {
        let child = self
            .child
            .as_ref()
            .context(STARTUP_CHILD_UNAVAILABLE_CONTEXT)?;
        let direct = DirectChildIdentity::capture_candidate(child)?;
        direct.prove_current(child)?;
        Ok(direct)
    }

    pub(crate) fn settle_spawn_without_identity(
        &mut self,
        boundary: &WaitableChildSpawnBoundary<'_>,
    ) {
        let child = self.child.take().expect(STARTUP_CHILD_UNAVAILABLE_CONTEXT);
        let mut child = crate::process::retain_unidentified_child(boundary, child);
        crate::worker::pty::pty_spawn_failure::settle_child(&mut child);
        self.untracked_gone_committed = true;
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn bind_linux_containment(
        &mut self,
        mut containment: crate::process::linux_containment::LinuxContainmentLease,
        child: ProcessIdentity,
    ) -> Result<crate::process::linux_containment::LinuxContainmentRecord> {
        ensure!(self.identity == Some(child), CHILD_IDENTITY_MISMATCH_DETAIL);
        containment
            .migrate_bootstrap(child)
            .context("migrate PTY bootstrap into its exclusive cgroup")?;
        let tree = self
            .tree
            .as_ref()
            .context(super::STARTUP_TREE_UNAVAILABLE_CONTEXT)?;
        let record = containment.record().clone();
        tree.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .install_linux_containment(containment)?;
        Ok(record)
    }

    pub(crate) fn bind_identified_tree(
        &mut self,
        direct: DirectChildIdentity,
        tree: CapturedProcessTree,
    ) -> Result<()> {
        self.remember_direct_identity(&direct);
        ensure!(
            tree.root() == direct.root() && tree.live_root() == direct.live_root(),
            CHILD_IDENTITY_MISMATCH_DETAIL
        );
        let child = self
            .child
            .as_ref()
            .context(STARTUP_CHILD_UNAVAILABLE_CONTEXT)?;
        let binding = RootWaitAuthority::prove_owned_child(child, &tree)?;
        let child = self
            .child
            .take()
            .context(STARTUP_CHILD_UNAVAILABLE_CONTEXT)?;
        self.authority = Some(binding.bind(child));
        let tracked = new_tracked_tree(tree);
        self.tree = Some(Arc::clone(&tracked));
        self.monitor = Some(StartupScopeMonitor::start(tracked)?);
        Ok(())
    }

    pub(crate) fn remember_direct_identity(&mut self, direct: &DirectChildIdentity) {
        self.process_id = Some(direct.root().pid);
        self.identity = Some(direct.root());
        self.live_identity = Some(direct.live_root());
    }

    pub(crate) fn commit_bootstrap_gate_release(&mut self) -> Result<()> {
        self.untracked_reaper
            .as_ref()
            .context(BOOTSTRAP_FALLBACK_UNAVAILABLE_CONTEXT)?;
        self.unreleased_gate
            .as_ref()
            .context(BOOTSTRAP_FALLBACK_UNAVAILABLE_CONTEXT)?;
        self.unreleased_gate = None;
        Ok(())
    }
}

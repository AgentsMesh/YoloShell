use super::cleanup_issues::CleanupIssues;
use super::runtime_record_authority::BoundRuntimeRecord;
use super::*;
use anyhow::{Context, Result, bail};
use std::collections::HashSet;
use yolo_process_test_support::{
    ProcessIdentity as RuntimeProcessIdentity, ProcessIdentityObservation, ResilienceProcessTree,
};
use yolo_runtime::model::ExitRecord;

#[path = "cleanup_tree_update.rs"]
mod update;

const CAPTURE_TREE_CONTEXT: &str = "capture runtime cleanup process tree";
const SIGNAL_TREE_CONTEXT: &str = "signal runtime cleanup process tree";
const MISSING_TREE_CUSTODY_DETAIL: &str =
    "runtime cleanup root disappeared before process-tree custody was captured";
const UNKNOWN_TREE_ROOT_DETAIL: &str =
    "runtime cleanup process-tree root identity could not be observed";

#[derive(Default)]
pub(super) struct RuntimeProcessTrees {
    trees: Vec<RuntimeProcessTree>,
    settled: HashSet<RuntimeProcessIdentity>,
}

pub(super) struct RuntimeProcessTree {
    root: RuntimeProcessIdentity,
    tree: ResilienceProcessTree,
    #[cfg(target_os = "linux")]
    containment: Option<super::runtime_record_authority::LinuxRuntimeCleanup>,
}

impl RuntimeProcessTrees {
    pub(super) fn signal_roots(
        &mut self,
        requested: &HashSet<RuntimeProcessIdentity>,
        signal: i32,
    ) -> Result<()> {
        self.ensure_roots(requested.iter().copied())?;
        let mut issues = CleanupIssues::default();
        for tree in self
            .trees
            .iter_mut()
            .filter(|tree| requested.contains(&tree.root))
        {
            #[cfg(target_os = "linux")]
            if signal == libc::SIGKILL
                && let Some(containment) = &mut tree.containment
            {
                issues.record(containment.poll());
                continue;
            }
            issues.record(
                tree.tree
                    .terminate(signal)
                    .with_context(|| format!("{SIGNAL_TREE_CONTEXT}: {:?}", tree.root)),
            );
        }
        issues.finish()
    }

    pub(super) fn ensure_records(&mut self, records: &[BoundRuntimeRecord]) -> Result<()> {
        let mut roots = records
            .iter()
            .map(|record| runtime_child_identity(record.record()))
            .collect::<Vec<_>>();
        sort_identities(&mut roots);
        let update = self.prepare_roots(roots)?;
        self.commit(update);
        Ok(())
    }

    pub(super) fn ensure_cleanup_records(
        &mut self,
        paths: &Paths,
        records: &[BoundRuntimeRecord],
    ) -> Result<()> {
        let update = self.prepare_cleanup_records(paths, records)?;
        self.commit(update);
        Ok(())
    }

    pub(super) fn identities(&self) -> Vec<RuntimeProcessIdentity> {
        let mut identities = self
            .trees
            .iter()
            .flat_map(|tree| tree.tree.identities())
            .collect::<Vec<_>>();
        sort_identities(&mut identities);
        identities
    }

    fn ensure_roots(
        &mut self,
        roots: impl IntoIterator<Item = RuntimeProcessIdentity>,
    ) -> Result<()> {
        let update = self.prepare_roots(roots)?;
        self.commit(update);
        Ok(())
    }

    pub(super) fn has_root(&self, root: RuntimeProcessIdentity) -> bool {
        self.settled.contains(&root) || self.trees.iter().any(|tree| tree.root == root)
    }

    pub(super) fn discovery_is_settled(&self, roots: &HashSet<RuntimeProcessIdentity>) -> bool {
        roots.iter().all(|root| {
            self.settled.contains(root)
                || self
                    .trees
                    .iter()
                    .any(|tree| tree.root == *root && tree.discovery_is_settled())
        })
    }

    pub(super) fn commit_trees(&mut self, mut pending: Vec<RuntimeProcessTree>) {
        self.trees.append(&mut pending);
        self.trees.sort_by_key(|tree| identity_key(tree.root));
    }
}

pub(super) fn capture_root(root: RuntimeProcessIdentity) -> Result<RuntimeProcessTree> {
    match root.observe() {
        ProcessIdentityObservation::Matching => {}
        ProcessIdentityObservation::Gone => bail!("{MISSING_TREE_CUSTODY_DETAIL}: {root:?}"),
        ProcessIdentityObservation::Unknown => bail!("{UNKNOWN_TREE_ROOT_DETAIL}: {root:?}"),
    }
    let tree = ResilienceProcessTree::capture(root)
        .with_context(|| format!("{CAPTURE_TREE_CONTEXT}: {root:?}"))?;
    Ok(RuntimeProcessTree {
        root,
        tree,
        #[cfg(target_os = "linux")]
        containment: None,
    })
}

impl RuntimeProcessTree {
    fn discovery_is_settled(&self) -> bool {
        #[cfg(target_os = "linux")]
        if let Some(containment) = &self.containment {
            return containment.is_settled();
        }
        self.tree.cleanup_discovery_is_settled()
    }
}

pub(super) fn has_exact_exit_evidence(paths: &Paths, record: &RuntimeRecord) -> bool {
    !record.runtime_epoch.is_nil()
        && read_json::<ExitRecord>(&paths.exit_record(record.terminal_id))
            .is_ok_and(|exit| exit.runtime_epoch == record.runtime_epoch)
}

pub(super) fn runtime_child_identity(record: &RuntimeRecord) -> RuntimeProcessIdentity {
    RuntimeProcessIdentity {
        pid: record.child_pid,
        start_time: record.child_start_time,
    }
}

pub(super) fn runtime_worker_identity(record: &RuntimeRecord) -> RuntimeProcessIdentity {
    RuntimeProcessIdentity {
        pid: record.worker_pid,
        start_time: record.worker_start_time,
    }
}

pub(super) fn sort_identities(identities: &mut Vec<RuntimeProcessIdentity>) {
    identities.sort_by_key(|identity| identity_key(*identity));
    identities.dedup();
}

pub(super) fn identity_key(identity: RuntimeProcessIdentity) -> (u32, u64) {
    (identity.pid, identity.start_time)
}

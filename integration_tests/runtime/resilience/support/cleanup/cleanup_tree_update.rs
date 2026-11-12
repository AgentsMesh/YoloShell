use super::*;

pub(crate) struct RuntimeProcessTreesUpdate {
    trees: Vec<RuntimeProcessTree>,
    settled: Vec<RuntimeProcessIdentity>,
}

impl RuntimeProcessTrees {
    pub(crate) fn prepare_cleanup_records(
        &self,
        paths: &Paths,
        records: &[BoundRuntimeRecord],
    ) -> Result<RuntimeProcessTreesUpdate> {
        let mut records = records.iter().collect::<Vec<_>>();
        records.sort_by_key(|record| identity_key(runtime_child_identity(record.record())));
        let mut trees = Vec::new();
        let mut settled = Vec::new();
        let mut issues = CleanupIssues::default();
        for record in records {
            let root = runtime_child_identity(record.record());
            if self.has_root(root) {
                continue;
            }
            if root.observe() == ProcessIdentityObservation::Gone
                && has_exact_exit_evidence(paths, record.record())
            {
                settled.push(root);
            } else if let Some(tree) = issues.record(capture_record(record)) {
                trees.push(tree);
            }
        }
        issues.finish()?;
        Ok(RuntimeProcessTreesUpdate { trees, settled })
    }

    pub(super) fn prepare_roots(
        &self,
        roots: impl IntoIterator<Item = RuntimeProcessIdentity>,
    ) -> Result<RuntimeProcessTreesUpdate> {
        let mut roots = roots
            .into_iter()
            .filter(|root| !self.has_root(*root))
            .collect::<Vec<_>>();
        sort_identities(&mut roots);
        let mut trees = Vec::new();
        let mut issues = CleanupIssues::default();
        for root in roots {
            if let Some(tree) = issues.record(capture_root(root)) {
                trees.push(tree);
            }
        }
        issues.finish()?;
        Ok(RuntimeProcessTreesUpdate {
            trees,
            settled: Vec::new(),
        })
    }

    pub(crate) fn commit(&mut self, update: RuntimeProcessTreesUpdate) {
        self.commit_trees(update.trees);
        self.settled.extend(update.settled);
    }
}

fn capture_record(record: &BoundRuntimeRecord) -> Result<RuntimeProcessTree> {
    #[cfg(target_os = "linux")]
    {
        Ok(RuntimeProcessTree {
            root: runtime_child_identity(record.record()),
            tree: record.capture_cleanup_tree()?,
            containment: record.linux_cleanup_authority(),
        })
    }
    #[cfg(not(target_os = "linux"))]
    capture_root(runtime_child_identity(record.record()))
}

use super::*;

impl PowerLossFreezeBarrier {
    pub(in super::super) async fn capture(
        paths: &Paths,
        records: &[BoundRuntimeRecord],
        mut guardians: Vec<BoundCustody>,
    ) -> Result<Self> {
        #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
        for guardian in &mut guardians {
            guardian.recover(paths).await?;
        }
        #[cfg(not(all(target_os = "macos", feature = "adversarial-test-hooks")))]
        let _ = paths;
        let mut barrier = Self {
            workers: Vec::new(),
            guardians,
            trees: Vec::new(),
            targets: HashMap::new(),
            frozen: HashSet::new(),
            thaw_on_drop: HashSet::new(),
            phase: PowerLossBarrierPhase::Freezing,
        };
        for bound in records {
            barrier.capture_record(bound)?;
        }
        let guardian_targets = barrier
            .guardians
            .iter()
            .filter_map(|guardian| {
                guardian
                    .direct_target()
                    .map(|identity| (identity, guardian.terminal_id()))
            })
            .collect::<Vec<_>>();
        for (identity, terminal_id) in guardian_targets {
            barrier.insert_new_target(identity, PowerLossTargetOwner::Guardian(terminal_id))?;
        }
        sort_targets(&mut barrier.workers);
        barrier.trees.sort_by_key(trees::PowerLossProcessTree::key);
        Ok(barrier)
    }

    fn capture_record(&mut self, bound: &BoundRuntimeRecord) -> Result<()> {
        let terminal_id = bound.record().terminal_id;
        let worker = runtime_worker(bound);
        self.insert_new_target(worker, PowerLossTargetOwner::Worker(terminal_id))?;
        self.workers.push(worker);
        #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
        let tree = match self
            .guardians
            .iter_mut()
            .find(|guardian| guardian.terminal_id() == terminal_id)
        {
            Some(guardian) => trees::PowerLossProcessTree::capture_custody(
                terminal_id,
                runtime_child(bound),
                guardian.take_recovery()?,
            )?,
            None => trees::PowerLossProcessTree::capture(terminal_id, runtime_child(bound))?,
        };
        #[cfg(target_os = "linux")]
        let tree = trees::PowerLossProcessTree::capture_linux(bound)?;
        #[cfg(not(any(
            target_os = "linux",
            all(target_os = "macos", feature = "adversarial-test-hooks")
        )))]
        let tree = trees::PowerLossProcessTree::capture(terminal_id, runtime_child(bound))?;
        for identity in tree.identities() {
            self.register_tree_target(identity, PowerLossTargetOwner::RuntimeTree(terminal_id))?;
        }
        self.trees.push(tree);
        Ok(())
    }

    pub(in super::super) async fn freeze_all(&mut self, paths: &Paths) -> Result<()> {
        for worker in self.workers.clone() {
            self.freeze_exact(worker).await?;
        }
        for index in 0..self.guardians.len() {
            self.guardians[index].confirm_active(paths)?;
            if let Some(identity) = self.guardians[index].direct_target() {
                self.freeze_exact(identity).await?;
            }
            self.guardians[index].confirm_active(paths)?;
        }
        self.freeze_runtime_trees().await?;
        self.confirm_guardians(paths)?;
        self.ensure_all_frozen().await
    }
}

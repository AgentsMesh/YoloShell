use super::cleanup_issues::CleanupIssues;
use super::cleanup_tree::{
    RuntimeProcessTrees, runtime_child_identity, runtime_worker_identity, sort_identities,
};
use super::runtime_record_authority::BoundRuntimeRecord;
use super::*;
use anyhow::{Context, Result, anyhow};
use std::collections::HashSet;
use yolo_process_test_support::{
    ProcessIdentity as RuntimeProcessIdentity, ProcessIdentityObservation,
};

#[path = "cleanup_records.rs"]
mod cleanup_records;
use cleanup_records::deduplicate_records;
#[path = "cleanup_candidates.rs"]
mod candidates;
pub(super) use candidates::collect_runtime_records;
#[path = "cleanup_binding.rs"]
mod binding;
#[path = "cleanup_boot.rs"]
mod boot_authority;
#[cfg(test)]
#[path = "cleanup_test_access.rs"]
mod test_access;
#[path = "cleanup_wait_identities.rs"]
mod wait_identities;
use wait_identities::{signal_identity, wait_for_runtime_identities};

const RUNTIME_FORCE_EXIT_TIMEOUT: Duration = Duration::from_secs(3);
const CLEANUP_POLL_INTERVAL: Duration = Duration::from_millis(20);
const SURVIVING_IDENTITIES_DETAIL: &str = "runtime identities survived forced cleanup";
const SIGNAL_IDENTITY_CONTEXT: &str = "signal runtime identity";

pub(crate) struct RuntimeCleanup {
    records: Vec<BoundRuntimeRecord>,
    unbound: Vec<RuntimeRecord>,
    trees: RuntimeProcessTrees,
    issues: CleanupIssues,
}

impl RuntimeCleanup {
    pub(crate) fn capture(paths: &Paths) -> (Self, Result<()>) {
        Self::with_trees(paths, &[], RuntimeProcessTrees::default())
    }

    pub(super) fn with_trees(
        paths: &Paths,
        tracked: &[BoundRuntimeRecord],
        mut trees: RuntimeProcessTrees,
    ) -> (Self, Result<()>) {
        let (records, mut issues) = collect_runtime_records(paths, tracked);
        issues.record(trees.ensure_cleanup_records(paths, &records));
        let capture = issues.check();
        (
            Self {
                records,
                unbound: Vec::new(),
                trees,
                issues,
            },
            capture,
        )
    }

    pub(crate) fn request(&mut self) {
        self.issues
            .record(request_runtime_cleanup(&self.records, &mut self.trees));
    }

    pub(crate) fn force(&mut self) {
        self.issues
            .record(force_runtime_cleanup(&self.records, &mut self.trees));
    }

    pub(crate) fn rescan(&mut self, paths: &Paths) -> Result<()> {
        let (mut records, issues) = collect_runtime_records(paths, &self.records);
        let mut operation = issues;
        operation.record(deduplicate_records(&mut records));
        let update = operation.record(self.trees.prepare_cleanup_records(paths, &records));
        let result = operation.check();
        if result.is_ok()
            && let Some(update) = update
        {
            self.records = records;
            self.unbound.clear();
            self.trees.commit(update);
        }
        self.issues.merge(operation);
        result
    }

    pub(crate) fn finish(&mut self) -> Result<()> {
        self.issues
            .record(candidates::reject_unbound_records(&self.unbound));
        std::mem::take(&mut self.issues).finish()
    }
}

fn request_runtime_cleanup(
    records: &[BoundRuntimeRecord],
    trees: &mut RuntimeProcessTrees,
) -> Result<()> {
    let mut issues = CleanupIssues::default();
    let mut requested_workers = HashSet::new();
    for record in records {
        let worker = runtime_worker_identity(record.record());
        if !requested_workers.contains(&worker) && request_worker_termination(record.record()) {
            requested_workers.insert(worker);
        }
    }
    let children = records
        .iter()
        .filter(|record| !requested_workers.contains(&runtime_worker_identity(record.record())))
        .map(|record| runtime_child_identity(record.record()))
        .collect::<HashSet<_>>();
    issues.record(trees.signal_roots(&children, libc::SIGTERM));
    let mut identities = runtime_identities(records);
    identities.extend(trees.identities());
    sort_identities(&mut identities);
    wait_for_runtime_identities(&identities, RUNTIME_GRACEFUL_EXIT_TIMEOUT);
    issues.finish()
}

fn force_runtime_cleanup(
    records: &[BoundRuntimeRecord],
    trees: &mut RuntimeProcessTrees,
) -> Result<()> {
    let mut issues = CleanupIssues::default();
    let workers = runtime_worker_identities(records);
    let children = records
        .iter()
        .map(|record| runtime_child_identity(record.record()))
        .collect::<HashSet<_>>();
    let deadline = Instant::now() + RUNTIME_FORCE_EXIT_TIMEOUT;
    let mut killed_workers = HashSet::new();
    loop {
        issues.record(trees.signal_roots(&children, libc::SIGKILL));
        for worker in &workers {
            match signal_identity(*worker, libc::SIGKILL) {
                Ok(true) => {
                    killed_workers.insert(worker.pid);
                }
                Ok(false) => {}
                Err(error) => issues.record_error(Some(error)),
            }
        }
        let mut identities = workers.clone();
        identities.extend(children.iter().copied());
        identities.extend(trees.identities());
        sort_identities(&mut identities);
        if identities
            .iter()
            .all(|identity| identity.observe() == ProcessIdentityObservation::Gone)
            && trees.discovery_is_settled(&children)
        {
            for pid in killed_workers {
                discard_incomplete_coverage_profile(pid);
            }
            return issues.finish();
        }
        if Instant::now() >= deadline {
            let survivors = identities
                .into_iter()
                .filter(|identity| identity.observe() != ProcessIdentityObservation::Gone)
                .collect::<Vec<_>>();
            issues.record_error(Some(anyhow!(
                "{SURVIVING_IDENTITIES_DETAIL}: {survivors:?}"
            )));
            return issues.finish();
        }
        std::thread::sleep(CLEANUP_POLL_INTERVAL);
    }
}

fn runtime_identities(records: &[BoundRuntimeRecord]) -> Vec<RuntimeProcessIdentity> {
    let mut identities = runtime_worker_identities(records);
    identities.extend(
        records
            .iter()
            .map(|record| runtime_child_identity(record.record())),
    );
    sort_identities(&mut identities);
    identities
}

fn runtime_worker_identities(records: &[BoundRuntimeRecord]) -> Vec<RuntimeProcessIdentity> {
    let mut identities = records
        .iter()
        .map(|record| runtime_worker_identity(record.record()))
        .collect();
    sort_identities(&mut identities);
    identities
}

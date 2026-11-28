use super::super::cleanup_issues::CleanupIssues;
use super::super::cleanup_tree::{runtime_child_identity, runtime_worker_identity};
use super::super::runtime_record_authority::BoundRuntimeRecord;
use super::super::runtime_record_validation::{
    RuntimeRecordScan, readable_runtime_records, readable_runtime_records_for_boot,
};
use super::cleanup_records::deduplicate_records;
use super::*;
use anyhow::anyhow;
use yolo_process_test_support::ProcessIdentityObservation;
use yolo_runtime::model::ExitRecord;

pub(super) const UNBOUND_RUNTIME_DETAIL: &str =
    "runtime cleanup disk candidate has no trusted process authority";

pub(super) struct RuntimeRecordDiscovery {
    pub(super) records: Vec<BoundRuntimeRecord>,
    pub(super) unbound: Vec<RuntimeRecord>,
    pub(super) issues: CleanupIssues,
}

pub(crate) fn collect_runtime_records(
    paths: &Paths,
    tracked: &[BoundRuntimeRecord],
) -> (Vec<BoundRuntimeRecord>, CleanupIssues) {
    reject_unbound(discover_scanned_runtime_records(
        paths,
        readable_runtime_records(paths),
        tracked,
    ))
}

pub(super) fn discover_runtime_records_for_boot(
    paths: &Paths,
    expected_boot_id: &str,
    tracked: &[BoundRuntimeRecord],
) -> RuntimeRecordDiscovery {
    discover_scanned_runtime_records(
        paths,
        readable_runtime_records_for_boot(paths, expected_boot_id),
        tracked,
    )
}

fn discover_scanned_runtime_records(
    paths: &Paths,
    scan: RuntimeRecordScan,
    tracked: &[BoundRuntimeRecord],
) -> RuntimeRecordDiscovery {
    let mut records = tracked.to_vec();
    let mut unbound = Vec::new();
    let mut issues = CleanupIssues::default();
    issues.record_errors(scan.failures);
    for candidate in scan.records {
        if refresh_bound_candidate(&mut records, &candidate) || safely_settled(paths, &candidate) {
            continue;
        }
        unbound.push(candidate);
    }
    issues.record(deduplicate_records(&mut records));
    RuntimeRecordDiscovery {
        records,
        unbound,
        issues,
    }
}

pub(super) fn reject_unbound_records(records: &[RuntimeRecord]) -> Result<()> {
    let mut issues = CleanupIssues::default();
    for candidate in records {
        issues.record_error(Some(anyhow!(
            "{UNBOUND_RUNTIME_DETAIL}: session={}, worker={:?}, child={:?}",
            candidate.terminal_id,
            runtime_worker_identity(candidate),
            runtime_child_identity(candidate)
        )));
    }
    issues.finish()
}

pub(super) fn retain_unsettled_pending(
    paths: &Paths,
    discovered: &mut Vec<RuntimeRecord>,
    pending: &[RuntimeRecord],
) {
    for candidate in pending {
        if safely_settled(paths, candidate)
            || discovered
                .iter()
                .any(|current| current.has_same_process_identity(candidate))
        {
            continue;
        }
        discovered.push(candidate.clone());
    }
}

fn reject_unbound(discovery: RuntimeRecordDiscovery) -> (Vec<BoundRuntimeRecord>, CleanupIssues) {
    let RuntimeRecordDiscovery {
        records,
        unbound,
        mut issues,
    } = discovery;
    issues.record(reject_unbound_records(&unbound));
    (records, issues)
}

fn refresh_bound_candidate(records: &mut [BoundRuntimeRecord], candidate: &RuntimeRecord) -> bool {
    records.iter_mut().any(|bound| {
        let Ok(refreshed) = bound.refresh(candidate) else {
            return false;
        };
        *bound = refreshed;
        true
    })
}

fn safely_settled(paths: &Paths, candidate: &RuntimeRecord) -> bool {
    let identities_gone = [
        runtime_worker_identity(candidate),
        runtime_child_identity(candidate),
    ]
    .into_iter()
    .all(|identity| identity.observe() == ProcessIdentityObservation::Gone);
    identities_gone
        && !candidate.runtime_epoch.is_nil()
        && read_json::<ExitRecord>(&paths.exit_record(candidate.terminal_id))
            .is_ok_and(|exit| exit.runtime_epoch == candidate.runtime_epoch)
}

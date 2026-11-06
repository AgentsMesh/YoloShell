use super::*;
use anyhow::{Result, ensure};
use std::collections::HashSet;
use yolo_process_test_support::ProcessIdentityObservation;

const EXIT_INSPECTION_CONTEXT: &str = "inspect simulated power-loss exit evidence";
const EXIT_PRESENT_DETAIL: &str = "simulated power loss produced exit evidence";
const DUPLICATE_SESSION_DETAIL: &str = "simulated power-loss session identity is duplicated";
const DUPLICATE_TARGET_DETAIL: &str = "simulated power-loss process identity is aliased";
const WORKER_NOT_LIVE_DETAIL: &str = "simulated power-loss worker identity is not live";
const CHILD_NOT_LIVE_DETAIL: &str = "simulated power-loss child identity is not live";

pub(super) fn validate_targets(paths: &Paths, records: &[BoundRuntimeRecord]) -> Result<()> {
    let mut sessions = HashSet::new();
    let mut targets = HashSet::new();
    for bound in records {
        bound.verify_runtime_file()?;
        let record = bound.record();
        super::super::runtime_record_validation::validate_runtime_record(
            paths,
            record,
            Some(record.terminal_id),
        )?;
        let worker = runtime_worker(bound);
        let child = runtime_child(bound);
        ensure!(
            sessions.insert(record.terminal_id),
            "{DUPLICATE_SESSION_DETAIL}: {}",
            record.terminal_id,
        );
        ensure!(
            targets.insert(worker),
            "{DUPLICATE_TARGET_DETAIL}: {worker:?}",
        );
        ensure!(
            targets.insert(child),
            "{DUPLICATE_TARGET_DETAIL}: {child:?}",
        );
    }
    Ok(())
}

pub(super) fn assert_targets_live(records: &[BoundRuntimeRecord]) {
    for bound in records {
        let worker = runtime_worker(bound);
        let child = runtime_child(bound);
        assert_eq!(
            worker.observe(),
            ProcessIdentityObservation::Matching,
            "{WORKER_NOT_LIVE_DETAIL}: {worker:?}"
        );
        assert_eq!(
            child.observe(),
            ProcessIdentityObservation::Matching,
            "{CHILD_NOT_LIVE_DETAIL}: {child:?}"
        );
    }
}

pub(super) fn assert_exit_evidence_absent(paths: &Paths, records: &[BoundRuntimeRecord]) {
    for bound in records {
        let path = paths.exit_record(bound.record().terminal_id);
        assert!(
            !path.try_exists().expect(EXIT_INSPECTION_CONTEXT),
            "{EXIT_PRESENT_DETAIL}: {}",
            path.display()
        );
    }
}

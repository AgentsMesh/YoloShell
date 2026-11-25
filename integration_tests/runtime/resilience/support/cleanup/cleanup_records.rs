use super::*;
use anyhow::{Result, anyhow};
use std::collections::HashMap;

const CONFLICTING_RUNTIME_RECORD_DETAIL: &str =
    "runtime cleanup found conflicting records for one process identity";

pub(super) fn deduplicate_records(records: &mut Vec<BoundRuntimeRecord>) -> Result<()> {
    let mut indexes = HashMap::new();
    let mut deduplicated = Vec::with_capacity(records.len());
    let mut issues = CleanupIssues::default();
    for record in records.drain(..) {
        let key = runtime_key(&record);
        let Some(index) = indexes.get(&key).copied() else {
            indexes.insert(key, deduplicated.len());
            deduplicated.push(record);
            continue;
        };
        let retained: &mut BoundRuntimeRecord = &mut deduplicated[index];
        if let Ok(refreshed) = retained.refresh(record.record()) {
            *retained = refreshed;
        } else if record.refresh(retained.record()).is_err() {
            issues.record_error(Some(anyhow!(
                "{CONFLICTING_RUNTIME_RECORD_DETAIL}: {key:?}"
            )));
        }
    }
    *records = deduplicated;
    issues.finish()
}

fn runtime_key(record: &BoundRuntimeRecord) -> (RuntimeProcessIdentity, RuntimeProcessIdentity) {
    (
        runtime_worker_identity(record.record()),
        runtime_child_identity(record.record()),
    )
}

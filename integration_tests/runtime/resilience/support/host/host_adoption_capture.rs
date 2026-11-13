use super::*;
use yolo_process_test_support::ProcessIdentityObservation;

const CAPTURE_RECORD_DETAIL: &str = "capture expected surviving runtime";
const CAPTURE_RECORD_IDENTITY_DETAIL: &str = "expected surviving runtime record changed identity";
const CAPTURE_WORKER_UNKNOWN_DETAIL: &str = "expected surviving Worker identity is indeterminate";
const EXIT_EVIDENCE_DETAIL: &str = "inspect expected runtime exit evidence";
const EXIT_EVIDENCE_MISMATCH_DETAIL: &str = "expected runtime exit evidence changed identity";
const TERMINATION_EVIDENCE_DETAIL: &str = "inspect expected runtime termination evidence";
const TERMINATION_EVIDENCE_MISMATCH_DETAIL: &str =
    "expected runtime termination evidence changed identity";

impl Harness {
    pub(in crate::support) fn runtime_adoption_expectations(
        &self,
    ) -> Result<Vec<RuntimeAdoptionExpectation>, String> {
        let paths = self.paths();
        let tracked = self
            .runtimes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        tracked
            .iter()
            .filter_map(|bound| capture_expectation(&paths, bound).transpose())
            .collect()
    }
}

fn capture_expectation(
    paths: &Paths,
    bound: &runtime_record_authority::BoundRuntimeRecord,
) -> Result<Option<RuntimeAdoptionExpectation>, String> {
    let trusted = bound.record();
    match yolo_process_test_support::observe_process_identity(
        trusted.worker_pid,
        trusted.worker_start_time,
    ) {
        ProcessIdentityObservation::Gone => return Ok(None),
        ProcessIdentityObservation::Unknown => {
            return Err(format!(
                "{CAPTURE_WORKER_UNKNOWN_DETAIL}: session={}",
                trusted.terminal_id
            ));
        }
        ProcessIdentityObservation::Matching => {}
    }
    let candidate: RuntimeRecord = read_json(&paths.runtime_record(trusted.terminal_id))
        .map_err(|error| format!("{CAPTURE_RECORD_DETAIL}: {error}"))?;
    if candidate.terminal_id != trusted.terminal_id {
        return Err(format!(
            "{CAPTURE_RECORD_IDENTITY_DETAIL}: expected={}, received={}",
            trusted.terminal_id, candidate.terminal_id
        ));
    }
    if candidate.runtime_epoch != trusted.runtime_epoch {
        return Ok(None);
    }
    let refreshed = bound
        .refresh(&candidate)
        .map_err(|error| format!("{CAPTURE_RECORD_IDENTITY_DETAIL}: {error:#}"))?;
    if has_terminal_evidence(paths, refreshed.record())? {
        return Ok(None);
    }
    Ok(Some(RuntimeAdoptionExpectation::new(refreshed)))
}

fn has_terminal_evidence(paths: &Paths, record: &RuntimeRecord) -> Result<bool, String> {
    if paths
        .exit_record(record.terminal_id)
        .try_exists()
        .map_err(|error| format!("{EXIT_EVIDENCE_DETAIL}: {error}"))?
    {
        let exit: ExitRecord = read_json(&paths.exit_record(record.terminal_id))
            .map_err(|error| format!("{EXIT_EVIDENCE_DETAIL}: {error}"))?;
        if exit.runtime_epoch != record.runtime_epoch {
            return Err(format!(
                "{EXIT_EVIDENCE_MISMATCH_DETAIL}: session={}",
                record.terminal_id
            ));
        }
        return Ok(true);
    }
    if !paths
        .terminate_tombstone(record.terminal_id)
        .try_exists()
        .map_err(|error| format!("{TERMINATION_EVIDENCE_DETAIL}: {error}"))?
    {
        return Ok(false);
    }
    let tombstone: yolo_runtime::TerminationTombstone =
        read_json(&paths.terminate_tombstone(record.terminal_id))
            .map_err(|error| format!("{TERMINATION_EVIDENCE_DETAIL}: {error}"))?;
    tombstone
        .validate_for(record.terminal_id, record.runtime_epoch)
        .map_err(|error| format!("{TERMINATION_EVIDENCE_MISMATCH_DETAIL}: {error}"))?;
    Ok(true)
}

#[cfg(test)]
#[path = "host_adoption_capture_tests.rs"]
mod tests;

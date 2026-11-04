use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::process::MacosSecondaryCustody;
use crate::pty_custody::CustodyDurableRecord;
use anyhow::{Context, Result};

const CANDIDATE_DETAIL: &str = "initial guardian candidate is unavailable for settlement";

pub(super) fn progress(
    namespace: &NamespaceLease,
    paths: &Paths,
    custody: &MacosSecondaryCustody,
    prepared: &CustodyDurableRecord,
    prepared_identity: FileIdentity,
) -> Result<bool> {
    let candidate = prepared
        .prepared_guardian_candidate()
        .context(CANDIDATE_DETAIL)?;
    if !settle_processes(custody, &prepared.runtime)? {
        return Ok(false);
    }
    crate::custody_retirement::publish_prepared_exact(
        namespace,
        paths,
        custody,
        prepared,
        prepared_identity,
        candidate,
        yolo_claude_custody::settle_runtime_retirement,
    )?;
    Ok(true)
}

fn settle_processes(
    custody: &MacosSecondaryCustody,
    runtime: &crate::pty_custody::CustodyRuntimeIdentity,
) -> Result<bool> {
    super::settlement_failure::processes_settled(custody, runtime)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/settlement/settlement_initial_tests.rs"]
mod tests;

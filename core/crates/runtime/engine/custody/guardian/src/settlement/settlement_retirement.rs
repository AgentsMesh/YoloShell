use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::process::MacosSecondaryCustody;
use crate::pty_custody::{CustodyDurableRecord, CustodyGuardianBinding};
use anyhow::{Result, ensure};

pub(super) enum Progress {
    Pending,
    Retired,
}

pub(super) fn publish(
    namespace: &NamespaceLease,
    paths: &Paths,
    custody: &MacosSecondaryCustody,
    record: &CustodyDurableRecord,
    runtime: &crate::model::RuntimeRecord,
    binding: CustodyGuardianBinding,
) -> Result<Progress> {
    let lease = record
        .predecessor_lease()
        .ok_or_else(|| anyhow::anyhow!(super::settlement::RECORD_DETAIL))?;
    ensure!(
        lease.guardian == Some(binding),
        super::settlement::RECORD_DETAIL
    );
    ensure!(
        record.runtime.matches_record(runtime),
        super::settlement::RECORD_DETAIL
    );
    let guard = yolo_paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
        paths,
        namespace,
        record.runtime.terminal_id,
    )?;
    let current: CustodyDurableRecord = guard
        .namespace()
        .read_json(&paths.pty_custody(record.runtime.terminal_id))?;
    if current.retired_lease() != Some(lease)
        && !crate::worker::worker_startup_checkpoint_allows_guardian_retirement(
            &guard, paths, runtime,
        )?
    {
        return Ok(Progress::Pending);
    }
    crate::custody_retirement::publish_exact_with_guard(
        &guard,
        paths,
        custody,
        &record.runtime,
        lease,
        yolo_claude_custody::settle_runtime_retirement,
    )?;
    Ok(Progress::Retired)
}

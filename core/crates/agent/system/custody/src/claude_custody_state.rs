use super::types::*;
use anyhow::{Context, Result, ensure};

pub(super) fn arm(ledger: &mut CustodyLedger, spec: &CustodySpec) -> Result<()> {
    if let Some(record) = ledger
        .records
        .iter()
        .find(|record| record.spec.invocation_id == spec.invocation_id)
    {
        ensure!(record.spec == *spec, "Claude custody identity changed");
        ensure!(
            record.state == CustodyState::Armed
                && record.publication == CustodyPublication::Pending,
            "Claude custody is no longer armable"
        );
        return Ok(());
    }
    ensure!(
        ledger.records.len() < MAX_RECORDS,
        "Claude custody capacity is exhausted"
    );
    ledger.records.push(CustodyRecord {
        spec: spec.clone(),
        state: CustodyState::Armed,
        publication: CustodyPublication::Pending,
    });
    Ok(())
}

pub(super) fn activate(ledger: &mut CustodyLedger, spec: &CustodySpec) -> Result<()> {
    let record = ledger
        .records
        .iter_mut()
        .find(|record| record.spec.invocation_id == spec.invocation_id)
        .context("Claude custody record is unavailable")?;
    ensure!(record.spec == *spec, "Claude custody identity changed");
    ensure!(
        record.state == CustodyState::Armed,
        "Claude custody is not publishable"
    );
    match record.publication {
        CustodyPublication::Active => return Ok(()),
        CustodyPublication::Pending => {}
    }
    ensure!(
        identities(spec).all(|identity| {
            identity.relation() == crate::process::ProcessIdentityRelation::Matching
        }),
        "Claude custody publication identities are unavailable"
    );
    record.publication = CustodyPublication::Active;
    Ok(())
}

pub(super) fn has_active_startup_receipt(
    ledger: &CustodyLedger,
    boot_id: &str,
    target: crate::process::ProcessIdentity,
    worker: crate::process::ProcessIdentity,
) -> bool {
    ledger.records.iter().any(|record| {
        record.state == CustodyState::Armed
            && record.publication == CustodyPublication::Active
            && identity_matches(&record.spec.target, boot_id, target)
            && identity_matches(&record.spec.worker, boot_id, worker)
    })
}

pub(super) fn transition(
    ledger: &mut CustodyLedger,
    spec: &CustodySpec,
    next: CustodyState,
) -> Result<()> {
    let record = ledger
        .records
        .iter_mut()
        .find(|record| record.spec.invocation_id == spec.invocation_id)
        .context("Claude custody record is unavailable")?;
    ensure!(record.spec == *spec, "Claude custody identity changed");
    match (&record.state, &next) {
        (CustodyState::Settled, CustodyState::Settled)
        | (CustodyState::Indeterminate, CustodyState::Indeterminate) => return Ok(()),
        (CustodyState::Settled, _) => {
            anyhow::bail!("Claude custody terminal state cannot change")
        }
        _ => record.state = next,
    }
    Ok(())
}

pub(super) fn begin_worker_termination(
    ledger: &mut CustodyLedger,
    worker: &DurableProcessIdentity,
) -> Result<()> {
    for record in matching_active_worker_records(ledger, worker) {
        if record.state != CustodyState::Settled {
            record.state = CustodyState::Containing;
        }
    }
    Ok(())
}

pub(super) fn settle_worker_termination(
    ledger: &mut CustodyLedger,
    worker: &DurableProcessIdentity,
) -> Result<()> {
    for record in matching_active_worker_records(ledger, worker) {
        record.state = CustodyState::Settled;
    }
    Ok(())
}

pub(super) fn prepare_admission(ledger: &mut CustodyLedger) -> Result<Vec<String>> {
    let mut blocked = Vec::new();
    ledger.records.retain_mut(|record| {
        if custody_predates_current_boot(&record.spec) {
            return false;
        }
        if record.state == CustodyState::Settled
            && record.spec.target.relation() == crate::process::ProcessIdentityRelation::Gone
        {
            return false;
        }
        let target_gone =
            record.spec.target.relation() == crate::process::ProcessIdentityRelation::Gone;
        let worker_gone =
            record.spec.worker.relation() == crate::process::ProcessIdentityRelation::Gone;
        let guardians_gone =
            record.spec.guardians.iter().all(|guardian| {
                guardian.relation() == crate::process::ProcessIdentityRelation::Gone
            });
        if record.publication == CustodyPublication::Pending && target_gone && guardians_gone {
            return false;
        }
        if worker_gone && guardians_gone {
            record.state = CustodyState::Indeterminate;
        }
        blocked.push(format!(
            "Claude invocation {} has unresolved {:?} {:?} custody",
            record.spec.invocation_id, record.publication, record.state
        ));
        true
    });
    Ok(blocked)
}

fn identities(spec: &CustodySpec) -> impl Iterator<Item = &DurableProcessIdentity> {
    std::iter::once(&spec.target)
        .chain(std::iter::once(&spec.worker))
        .chain(spec.guardians.iter())
}

fn matching_active_worker_records<'a>(
    ledger: &'a mut CustodyLedger,
    worker: &'a DurableProcessIdentity,
) -> impl Iterator<Item = &'a mut CustodyRecord> {
    ledger.records.iter_mut().filter(|record| {
        record.publication == CustodyPublication::Active && record.spec.worker == *worker
    })
}

fn identity_matches(
    durable: &DurableProcessIdentity,
    boot_id: &str,
    process: crate::process::ProcessIdentity,
) -> bool {
    durable.boot_id == boot_id
        && durable.pid == process.pid
        && durable.start_time == process.start_time
}

fn custody_predates_current_boot(spec: &CustodySpec) -> bool {
    let current = crate::process::boot_id();
    identities(spec).all(|identity| {
        crate::process::boot_identity_relation(&identity.boot_id, &current)
            == crate::process::BootIdentityRelation::Different
    })
}

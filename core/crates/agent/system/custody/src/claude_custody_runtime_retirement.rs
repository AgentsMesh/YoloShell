use super::context::ManagedCustodySession;
use super::types::{
    CustodyLedger, CustodyPublication, CustodyState, DurableProcessIdentity, MAX_LEDGER_BYTES,
};
use crate::custody_retirement::VerifiedRuntimeRetirement;
use anyhow::{Result, ensure};

const RECORDS_UNSETTLED: &str = "Claude runtime retirement retains unresolved custody identities";

pub fn settle_runtime_retirement(authority: &VerifiedRuntimeRetirement<'_, '_>) -> Result<()> {
    authority.verify()?;
    let path = authority.session_directory().join(super::LEDGER_FILE);
    let guard = authority.mutation_guard();
    guard.require_no_pending_intent()?;
    let Some((mut ledger, identity)) = guard.read_optional_with_identity::<CustodyLedger>(&path)?
    else {
        return authority.verify();
    };
    let session = ManagedCustodySession::capture(&path)?;
    session.require_current_manifest(guard)?;
    ensure!(
        identity.length() <= MAX_LEDGER_BYTES,
        "{}",
        super::persistence::LEDGER_TOO_LARGE
    );
    super::persistence::validate(&ledger)?;
    let worker = DurableProcessIdentity::new(authority.boot_id().to_owned(), authority.worker());
    verify_participants(&ledger, &worker)?;
    let changed = settle_matching_records(&mut ledger, &worker);
    if !changed {
        return authority.verify();
    }
    let verify = || {
        authority.verify()?;
        session.require_current_manifest(guard)?;
        verify_participants(&ledger, &worker)
    };
    verify()?;
    guard.write_json_expected_with_fence_identity(&path, Some(identity), &ledger, |_| verify())?;
    verify()
}

fn verify_participants(ledger: &CustodyLedger, worker: &DurableProcessIdentity) -> Result<()> {
    for record in ledger.records.iter().filter(|record| {
        record.publication == CustodyPublication::Active && record.spec.worker == *worker
    }) {
        for participant in std::iter::once(&record.spec.target).chain(record.spec.guardians.iter())
        {
            ensure!(
                participant.boot_id == worker.boot_id
                    && participant.relation() == crate::process::ProcessIdentityRelation::Gone,
                RECORDS_UNSETTLED
            );
        }
    }
    Ok(())
}

fn settle_matching_records(ledger: &mut CustodyLedger, worker: &DurableProcessIdentity) -> bool {
    let mut changed = false;
    for record in &mut ledger.records {
        if record.publication == CustodyPublication::Active
            && record.spec.worker == *worker
            && record.state != CustodyState::Settled
        {
            record.state = CustodyState::Settled;
            changed = true;
        }
    }
    changed
}

#[cfg(all(test, feature = "yolo-integration-tests", target_os = "macos"))]
#[path = "../integration_tests/runtime_retirement.rs"]
mod integration_tests;

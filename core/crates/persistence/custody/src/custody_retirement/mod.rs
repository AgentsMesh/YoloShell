use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::process::MacosSecondaryCustody;
use crate::pty_custody::{CustodyDurableLease, CustodyDurableRecord, CustodyRuntimeIdentity};
use anyhow::{Context, Result, ensure};
use yolo_paths::durable_mutation::{DurableMutationFencePhase, SessionDurableMutationGuard};

#[path = "prepared.rs"]
mod prepared;
#[path = "proof.rs"]
mod proof;
mod publication;
pub use publication::publish_exact_with_guard;
mod authority;
pub use authority::VerifiedRuntimeRetirement;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../test_support/custody_retirement/test_hook.rs"]
mod test_hook;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/custody_retirement/tests.rs"]
mod tests;

pub use prepared::publish_exact as publish_prepared_exact;

const CUSTODY_DETAIL: &str = "durable PTY custody retirement predecessor changed";
const RECEIPT_DETAIL: &str = "durable PTY custody retirement receipt is not exact";

pub(crate) fn publish_exact(
    namespace: &NamespaceLease,
    paths: &Paths,
    custody: &MacosSecondaryCustody,
    runtime: &CustodyRuntimeIdentity,
    expected_lease: CustodyDurableLease,
) -> Result<()> {
    validate_claim(runtime, expected_lease)?;
    let guard =
        SessionDurableMutationGuard::acquire_participant(paths, namespace, runtime.terminal_id)?;
    publish_exact_with_guard(&guard, paths, custody, runtime, expected_lease, |_| Ok(()))
}

fn verify_durable_receipt(
    proof: &proof::RetirementProof,
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    custody: &MacosSecondaryCustody,
    runtime: &CustodyRuntimeIdentity,
    expected_lease: CustodyDurableLease,
    path: &std::path::Path,
    record: &CustodyDurableRecord,
    identity: FileIdentity,
) -> Result<()> {
    verify_receipt(
        proof,
        guard,
        paths,
        custody,
        runtime,
        expected_lease,
        record,
    )?;
    let parent = path.parent().context(RECEIPT_DETAIL)?;
    guard.namespace().directory(parent)?.sync()?;
    #[cfg(test)]
    test_hook::run_after_receipt_parent_sync()?;
    let current: (CustodyDurableRecord, FileIdentity) =
        guard.namespace().read_json_with_identity(path)?;
    ensure!(current == (record.clone(), identity), RECEIPT_DETAIL);
    verify_receipt(
        proof,
        guard,
        paths,
        custody,
        runtime,
        expected_lease,
        &current.0,
    )
}

fn validate_claim(runtime: &CustodyRuntimeIdentity, lease: CustodyDurableLease) -> Result<()> {
    let binding = lease.guardian.context(CUSTODY_DETAIL)?;
    ensure!(
        runtime.is_complete() && binding.matches_durable_lease(runtime, &lease),
        CUSTODY_DETAIL
    );
    Ok(())
}

fn verify_receipt(
    proof: &proof::RetirementProof,
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    custody: &MacosSecondaryCustody,
    runtime: &CustodyRuntimeIdentity,
    expected_lease: CustodyDurableLease,
    record: &CustodyDurableRecord,
) -> Result<()> {
    ensure!(
        receipt_is_exact(record, runtime, expected_lease, proof.evidence()),
        RECEIPT_DETAIL
    );
    proof.verify(guard, paths, custody)
}

fn receipt_is_exact(
    record: &CustodyDurableRecord,
    runtime: &CustodyRuntimeIdentity,
    expected_lease: CustodyDurableLease,
    evidence: crate::pty_custody::CustodyRetirementEvidence,
) -> bool {
    record.is_valid()
        && record.runtime == *runtime
        && record.retired_lease() == Some(expected_lease)
        && record.retirement_evidence() == Some(evidence)
}

fn verify_transition_phase(
    namespace: &NamespaceLease,
    path: &std::path::Path,
    predecessor: &CustodyDurableRecord,
    predecessor_identity: FileIdentity,
    retired: &CustodyDurableRecord,
    phase: DurableMutationFencePhase,
) -> Result<()> {
    let current: (CustodyDurableRecord, FileIdentity) = namespace.read_json_with_identity(path)?;
    match phase {
        DurableMutationFencePhase::Commit => ensure!(
            current == (predecessor.clone(), predecessor_identity),
            CUSTODY_DETAIL
        ),
        DurableMutationFencePhase::Installed => ensure!(current.0 == *retired, RECEIPT_DETAIL),
    }
    Ok(())
}

use super::proof::RetirementProof;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::process::MacosSecondaryCustody;
use crate::pty_custody::{CustodyDurableLease, CustodyDurableRecord};
use anyhow::{Context, Result, ensure};
use yolo_paths::durable_mutation::{DurableMutationFencePhase, SessionDurableMutationGuard};

const PREPARED_DETAIL: &str = "prepared PTY custody retirement predecessor changed";
const RECEIPT_DETAIL: &str = "prepared PTY custody retirement receipt is not exact";

pub fn publish_exact(
    namespace: &NamespaceLease,
    paths: &Paths,
    custody: &MacosSecondaryCustody,
    prepared: &CustodyDurableRecord,
    prepared_identity: FileIdentity,
    candidate: CustodyDurableLease,
    settle: impl FnOnce(&super::VerifiedRuntimeRetirement<'_, '_>) -> Result<()>,
) -> Result<()> {
    publish_exact_observed(
        namespace,
        paths,
        custody,
        prepared,
        prepared_identity,
        candidate,
        settle,
        |_| {},
    )
}

fn publish_exact_observed<F>(
    namespace: &NamespaceLease,
    paths: &Paths,
    custody: &MacosSecondaryCustody,
    prepared: &CustodyDurableRecord,
    prepared_identity: FileIdentity,
    candidate: CustodyDurableLease,
    settle: impl FnOnce(&super::VerifiedRuntimeRetirement<'_, '_>) -> Result<()>,
    after_commit: F,
) -> Result<()>
where
    F: FnMut(FileIdentity),
{
    validate_claim(prepared, candidate)?;
    let guard = SessionDurableMutationGuard::acquire_participant(
        paths,
        namespace,
        prepared.runtime.terminal_id,
    )?;
    let path = paths.pty_custody(prepared.runtime.terminal_id);
    require_prepared(&guard, &path, prepared, prepared_identity)?;
    let proof = RetirementProof::capture(&guard, paths, custody, &prepared.runtime, candidate)?;
    super::VerifiedRuntimeRetirement::new(
        &guard,
        paths,
        custody,
        &proof,
        prepared,
        prepared_identity,
    )?
    .settle(settle)?;
    let retired = prepared
        .retire_prepared_guardian_with_evidence(candidate, proof.evidence())
        .context(PREPARED_DETAIL)?;
    let installed_identity = guard.write_json_expected_with_fence_observer(
        &path,
        Some(prepared_identity),
        &retired,
        |phase| {
            proof.verify(&guard, paths, custody)?;
            verify_phase(&guard, &path, prepared, prepared_identity, &retired, phase)
        },
        after_commit,
    )?;
    proof.verify(&guard, paths, custody)?;
    let installed: (CustodyDurableRecord, FileIdentity) =
        namespace.read_json_with_identity(&path)?;
    ensure!(installed == (retired, installed_identity), RECEIPT_DETAIL);
    Ok(())
}

fn validate_claim(prepared: &CustodyDurableRecord, candidate: CustodyDurableLease) -> Result<()> {
    let binding = candidate.guardian.context(PREPARED_DETAIL)?;
    ensure!(
        prepared.is_valid()
            && prepared.prepared_guardian_candidate() == Some(candidate)
            && binding.matches_durable_lease(&prepared.runtime, &candidate),
        PREPARED_DETAIL
    );
    Ok(())
}

fn require_prepared(
    guard: &SessionDurableMutationGuard<'_>,
    path: &std::path::Path,
    prepared: &CustodyDurableRecord,
    identity: FileIdentity,
) -> Result<()> {
    let current: (CustodyDurableRecord, FileIdentity) =
        guard.namespace().read_json_with_identity(path)?;
    ensure!(current == (prepared.clone(), identity), PREPARED_DETAIL);
    Ok(())
}

fn verify_phase(
    guard: &SessionDurableMutationGuard<'_>,
    path: &std::path::Path,
    prepared: &CustodyDurableRecord,
    prepared_identity: FileIdentity,
    retired: &CustodyDurableRecord,
    phase: DurableMutationFencePhase,
) -> Result<()> {
    let current: (CustodyDurableRecord, FileIdentity) =
        guard.namespace().read_json_with_identity(path)?;
    match phase {
        DurableMutationFencePhase::Commit => {
            ensure!(
                current == (prepared.clone(), prepared_identity),
                PREPARED_DETAIL
            )
        }
        DurableMutationFencePhase::Installed => {
            ensure!(current.0 == *retired, RECEIPT_DETAIL)
        }
    }
    Ok(())
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/custody_retirement/prepared_tests.rs"]
mod tests;

use super::*;

pub fn publish_exact_with_guard(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    custody: &MacosSecondaryCustody,
    runtime: &CustodyRuntimeIdentity,
    expected_lease: CustodyDurableLease,
    settle: impl FnOnce(&VerifiedRuntimeRetirement<'_, '_>) -> Result<()>,
) -> Result<()> {
    validate_claim(runtime, expected_lease)?;
    ensure!(guard.terminal_id() == runtime.terminal_id, CUSTODY_DETAIL);
    let proof = proof::RetirementProof::capture(guard, paths, custody, runtime, expected_lease)?;
    let path = paths.pty_custody(runtime.terminal_id);
    let (current, identity): (CustodyDurableRecord, FileIdentity) =
        guard.namespace().read_json_with_identity(&path)?;
    if current.retired_lease() == Some(expected_lease) {
        verify_durable_receipt(
            &proof,
            guard,
            paths,
            custody,
            runtime,
            expected_lease,
            &path,
            &current,
            identity,
        )?;
        return VerifiedRuntimeRetirement::new(guard, paths, custody, &proof, &current, identity)?
            .settle(settle);
    }
    ensure!(
        current.is_valid()
            && current.runtime == *runtime
            && current.armed_lease() == Some(expected_lease),
        CUSTODY_DETAIL
    );
    // A terminal PTY receipt must never become durable before provider custody settles.
    VerifiedRuntimeRetirement::new(guard, paths, custody, &proof, &current, identity)?
        .settle(settle)?;
    let retired = current
        .retire_with_evidence(expected_lease, proof.evidence())
        .context(CUSTODY_DETAIL)?;
    let installed_identity = guard.write_json_expected_with_fence_identity(
        &path,
        Some(identity),
        &retired,
        |phase| {
            proof.verify(guard, paths, custody)?;
            verify_transition_phase(
                guard.namespace(),
                &path,
                &current,
                identity,
                &retired,
                phase,
            )
        },
    )?;
    proof.verify(guard, paths, custody)?;
    let installed: (CustodyDurableRecord, FileIdentity) =
        guard.namespace().read_json_with_identity(&path)?;
    ensure!(installed == (retired, installed_identity), RECEIPT_DETAIL);
    Ok(())
}

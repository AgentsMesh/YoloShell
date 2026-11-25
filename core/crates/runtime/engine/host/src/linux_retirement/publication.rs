use super::{Host, RecoverySnapshot, receipt::RetirementProof, require_gone_runtime_processes};
use crate::paths::identity::FileIdentity;
use crate::pty_custody::CustodyDurableRecord;
use anyhow::{Result, ensure};
use yolo_paths::durable_mutation::{DurableMutationFencePhase, SessionDurableMutationGuard};

const RECEIPT_DETAIL: &str = "Linux retirement publication changed its exact custody receipt";

pub(super) fn publish(
    host: &Host,
    guard: &SessionDurableMutationGuard<'_>,
    snapshot: &RecoverySnapshot,
    proof: &RetirementProof,
) -> Result<()> {
    let lease = super::receipt::validate_lease(host, snapshot)?;
    let retired = snapshot
        .custody
        .retire_with_evidence(lease, snapshot.evidence(lease))
        .ok_or_else(|| anyhow::anyhow!(RECEIPT_DETAIL))?;
    let path = host.paths.pty_custody(snapshot.runtime.terminal_id);
    snapshot.verify(host)?;
    verify_settlement(host, guard, snapshot, proof)?;
    let installed_identity = guard.write_json_expected_with_fence_identity(
        &path,
        Some(snapshot.custody_identity),
        &retired,
        |phase| {
            verify_settlement(host, guard, snapshot, proof)?;
            match phase {
                DurableMutationFencePhase::Commit => snapshot.verify_custody(host)?,
                DurableMutationFencePhase::Installed => {
                    let current: CustodyDurableRecord = host.namespace.read_json(&path)?;
                    ensure!(current == retired, RECEIPT_DETAIL);
                }
            }
            Ok(())
        },
    )?;
    verify_settlement(host, guard, snapshot, proof)?;
    let installed: (CustodyDurableRecord, FileIdentity) =
        host.namespace.read_json_with_identity(&path)?;
    ensure!(installed == (retired, installed_identity), RECEIPT_DETAIL);
    Ok(())
}

pub(super) fn confirm(
    host: &Host,
    guard: &SessionDurableMutationGuard<'_>,
    snapshot: &RecoverySnapshot,
    proof: &RetirementProof,
) -> Result<()> {
    snapshot.validate_receipt(host)?;
    verify_settlement(host, guard, snapshot, proof)?;
    host.namespace
        .directory(&host.paths.session_dir(snapshot.runtime.terminal_id))?
        .sync()?;
    snapshot.verify(host)?;
    verify_settlement(host, guard, snapshot, proof)
}

fn verify_settlement(
    host: &Host,
    guard: &SessionDurableMutationGuard<'_>,
    snapshot: &RecoverySnapshot,
    proof: &RetirementProof,
) -> Result<()> {
    guard.require_no_pending_intent()?;
    ensure!(
        guard.terminal_id() == snapshot.runtime.terminal_id,
        RECEIPT_DETAIL
    );
    snapshot.verify_source(host)?;
    require_gone_runtime_processes(&snapshot.runtime)?;
    proof.verify(host, snapshot)
}

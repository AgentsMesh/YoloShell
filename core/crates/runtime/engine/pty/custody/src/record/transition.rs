use crate::paths::identity::FileIdentity;
use crate::pty_custody::CustodyDurableRecord;
use anyhow::{Result, ensure};
use std::path::Path;
use yolo_paths::durable_mutation::{DurableMutationFencePhase, SessionDurableMutationGuard};

const TRANSITION_DETAIL: &str = "durable PTY custody transition lost its exact predecessor";

pub fn replace_exact(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    expected: &CustodyDurableRecord,
    expected_identity: FileIdentity,
    replacement: &CustodyDurableRecord,
) -> Result<FileIdentity> {
    replace_exact_observed(
        guard,
        path,
        expected,
        expected_identity,
        replacement,
        |_| {},
    )
}

fn validate_transition(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    expected: &CustodyDurableRecord,
    expected_identity: FileIdentity,
    replacement: &CustodyDurableRecord,
) -> Result<()> {
    ensure!(
        expected.is_valid()
            && replacement.is_valid()
            && expected.runtime == replacement.runtime
            && expected != replacement,
        TRANSITION_DETAIL
    );
    let current: (CustodyDurableRecord, FileIdentity) =
        guard.namespace().read_json_with_identity(path)?;
    ensure!(
        current == (expected.clone(), expected_identity),
        TRANSITION_DETAIL
    );
    Ok(())
}

fn verify_installed(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    replacement: &CustodyDurableRecord,
    installed_identity: FileIdentity,
) -> Result<()> {
    let installed: (CustodyDurableRecord, FileIdentity) =
        guard.namespace().read_json_with_identity(path)?;
    ensure!(
        installed == (replacement.clone(), installed_identity),
        TRANSITION_DETAIL
    );
    Ok(())
}

pub fn replace_exact_observed<F>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    expected: &CustodyDurableRecord,
    expected_identity: FileIdentity,
    replacement: &CustodyDurableRecord,
    after_commit: F,
) -> Result<FileIdentity>
where
    F: FnMut(FileIdentity),
{
    replace_exact_observed_with_fence(
        guard,
        path,
        expected,
        expected_identity,
        replacement,
        after_commit,
        |_| Ok(()),
    )
}

pub fn replace_exact_observed_with_fence<F, G>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    expected: &CustodyDurableRecord,
    expected_identity: FileIdentity,
    replacement: &CustodyDurableRecord,
    after_commit: F,
    mut additional_fence: G,
) -> Result<FileIdentity>
where
    F: FnMut(FileIdentity),
    G: FnMut(DurableMutationFencePhase) -> Result<()>,
{
    validate_transition(guard, path, expected, expected_identity, replacement)?;
    let identity = guard.write_json_expected_with_fence_observer(
        path,
        Some(expected_identity),
        replacement,
        |phase| {
            verify_phase(guard, path, expected, replacement, expected_identity, phase)?;
            additional_fence(phase)
        },
        after_commit,
    )?;
    verify_installed(guard, path, replacement, identity)?;
    Ok(identity)
}

fn verify_phase(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
    expected: &CustodyDurableRecord,
    replacement: &CustodyDurableRecord,
    expected_identity: FileIdentity,
    phase: DurableMutationFencePhase,
) -> Result<()> {
    let current: (CustodyDurableRecord, FileIdentity) =
        guard.namespace().read_json_with_identity(path)?;
    match phase {
        DurableMutationFencePhase::Commit => {
            ensure!(
                current == (expected.clone(), expected_identity),
                TRANSITION_DETAIL
            )
        }
        DurableMutationFencePhase::Installed => {
            ensure!(current.0 == *replacement, TRANSITION_DETAIL)
        }
    }
    Ok(())
}

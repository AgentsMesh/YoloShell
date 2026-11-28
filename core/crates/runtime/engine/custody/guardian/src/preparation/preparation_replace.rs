use super::{PREPARATION_DETAIL, reconcile};
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::CustodyDurableRecord;
use anyhow::Result;
use yolo_paths::durable_mutation::SessionDurableMutationGuard;

pub(super) fn replace(
    namespace: &NamespaceLease,
    paths: &Paths,
    expected: &CustodyDurableRecord,
    expected_identity: FileIdentity,
    replacement: &CustodyDurableRecord,
) -> Result<FileIdentity> {
    replace_observed(
        namespace,
        paths,
        expected,
        expected_identity,
        replacement,
        |_| {},
    )
}

pub(super) fn replace_observed<F>(
    namespace: &NamespaceLease,
    paths: &Paths,
    expected: &CustodyDurableRecord,
    expected_identity: FileIdentity,
    replacement: &CustodyDurableRecord,
    after_commit: F,
) -> Result<FileIdentity>
where
    F: FnMut(FileIdentity),
{
    let guard = SessionDurableMutationGuard::acquire_participant(
        paths,
        namespace,
        expected.runtime.terminal_id,
    )?;
    replace_guarded_observed(
        &guard,
        paths,
        expected,
        expected_identity,
        replacement,
        after_commit,
    )
}

pub(super) fn replace_guarded(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    expected: &CustodyDurableRecord,
    expected_identity: FileIdentity,
    replacement: &CustodyDurableRecord,
) -> Result<FileIdentity> {
    replace_guarded_observed(
        guard,
        paths,
        expected,
        expected_identity,
        replacement,
        |_| {},
    )
}

fn replace_guarded_observed<F>(
    guard: &SessionDurableMutationGuard<'_>,
    paths: &Paths,
    expected: &CustodyDurableRecord,
    expected_identity: FileIdentity,
    replacement: &CustodyDurableRecord,
    mut after_commit: F,
) -> Result<FileIdentity>
where
    F: FnMut(FileIdentity),
{
    anyhow::ensure!(
        guard.terminal_id() == expected.runtime.terminal_id,
        PREPARATION_DETAIL
    );
    let path = paths.pty_custody(expected.runtime.terminal_id);
    let committed = std::cell::RefCell::new(None);
    let installed = crate::pty_custody::replace_exact_record_observed(
        guard,
        &path,
        expected,
        expected_identity,
        replacement,
        |staged_identity| {
            let observed = observe_replacement(guard, &path, replacement, staged_identity);
            let identity = observed.as_ref().ok().copied();
            committed.replace(Some(observed));
            if let Some(identity) = identity {
                after_commit(identity);
            }
        },
    );
    settle_replacement(guard, &path, replacement, installed, committed.into_inner())
}

fn observe_replacement(
    guard: &SessionDurableMutationGuard<'_>,
    path: &std::path::Path,
    replacement: &CustodyDurableRecord,
    staged_identity: FileIdentity,
) -> Result<FileIdentity> {
    let current: (CustodyDurableRecord, FileIdentity) =
        guard.namespace().read_json_with_identity(path)?;
    anyhow::ensure!(
        current.0 == *replacement && current.1.same_object(staged_identity),
        PREPARATION_DETAIL
    );
    Ok(current.1)
}

fn settle_replacement(
    guard: &SessionDurableMutationGuard<'_>,
    path: &std::path::Path,
    replacement: &CustodyDurableRecord,
    installed: Result<FileIdentity>,
    committed: Option<Result<FileIdentity>>,
) -> Result<FileIdentity> {
    match (installed, committed) {
        (Ok(identity), Some(Ok(observed))) => {
            anyhow::ensure!(identity == observed, PREPARATION_DETAIL);
            Ok(identity)
        }
        (Ok(_), Some(Err(observation))) => Err(observation),
        (Ok(_), None) => anyhow::bail!(PREPARATION_DETAIL),
        (Err(original), Some(Ok(observed))) => {
            reconcile::committed(guard, path, replacement, Some(observed))
                .map_err(|reconcile| original.context(reconcile))
        }
        (Err(original), Some(Err(observation))) => Err(original.context(observation)),
        (Err(original), None) => Err(original),
    }
}

use super::{WorkerNamespace, secondary_custody_publication_fence, startup_checkpoint};
use crate::model::RuntimeRecord;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::CustodyDurableRecord;
use anyhow::{Context, Result, ensure};

#[path = "secondary_custody_persistence_guard.rs"]
mod guard;
#[path = "secondary_custody_persistence_reconcile.rs"]
mod reconcile;

const MANIFEST_CONTEXT: &str = "read PTY custody session manifest";
const MANIFEST_DETAIL: &str = "PTY custody requires the current non-archived session manifest";
const RUNTIME_IDENTITY_DETAIL: &str = "PTY custody record does not match its runtime";

pub(crate) fn publish_pending(
    namespace: &WorkerNamespace,
    paths: &Paths,
    runtime: &RuntimeRecord,
    checkpoint: &startup_checkpoint::ActiveStartupCheckpoint,
) -> Result<(CustodyDurableRecord, FileIdentity)> {
    let custody = CustodyDurableRecord::pending(
        crate::pty_custody::CustodyRuntimeIdentity::from_record(runtime),
    );
    let identity = persist_pending(namespace, paths, runtime, &custody, checkpoint)?;
    Ok((custody, identity))
}

fn persist_pending(
    namespace: &WorkerNamespace,
    paths: &Paths,
    runtime: &RuntimeRecord,
    custody: &CustodyDurableRecord,
    checkpoint: &startup_checkpoint::ActiveStartupCheckpoint,
) -> Result<FileIdentity> {
    validate_custody(runtime, custody)?;
    let guard = guard::durable_guard(namespace, paths, runtime.terminal_id)?;
    validate_manifest(namespace, paths, runtime)?;
    let path = paths.pty_custody(runtime.terminal_id);
    let fence = secondary_custody_publication_fence::PublicationFence::capture(
        namespace, paths, runtime, custody, checkpoint,
    )?;
    if fence.already_installed(namespace, paths, runtime, custody, checkpoint)? {
        return exact_identity(namespace, &path, custody);
    }
    let expected = fence.expected_custody_identity();
    let identity =
        guard.write_json_expected_with_fence_identity(&path, expected, custody, |phase| {
            fence.require_exact(namespace, paths, runtime, custody, checkpoint, phase)
        })?;
    namespace.verify()?;
    ensure!(
        exact_identity(namespace, &path, custody)? == identity,
        RUNTIME_IDENTITY_DETAIL
    );
    Ok(identity)
}

pub(crate) fn persist_transition(
    namespace: &WorkerNamespace,
    paths: &Paths,
    runtime: &RuntimeRecord,
    runtime_identity: FileIdentity,
    expected: &CustodyDurableRecord,
    expected_identity: FileIdentity,
    replacement: &CustodyDurableRecord,
) -> Result<()> {
    persist_transition_with_boundary(
        namespace,
        paths,
        runtime,
        runtime_identity,
        expected,
        expected_identity,
        replacement,
        || Ok(()),
    )
    .map(|_| ())
}

pub(crate) fn persist_transition_with_boundary<F>(
    namespace: &WorkerNamespace,
    paths: &Paths,
    runtime: &RuntimeRecord,
    runtime_identity: FileIdentity,
    expected: &CustodyDurableRecord,
    expected_identity: FileIdentity,
    replacement: &CustodyDurableRecord,
    after_install: F,
) -> Result<FileIdentity>
where
    F: FnOnce() -> Result<()>,
{
    validate_custody(runtime, expected)?;
    validate_custody(runtime, replacement)?;
    let guard = guard::durable_guard(namespace, paths, runtime.terminal_id)?;
    validate_manifest(namespace, paths, runtime)?;
    let path = paths.pty_custody(runtime.terminal_id);
    let runtime_path = paths.runtime_record(runtime.terminal_id);
    reconcile::require_runtime_exact(guard.namespace(), &runtime_path, runtime, runtime_identity)?;
    let committed = std::cell::Cell::new(None);
    let installed_identity = std::cell::Cell::new(None);
    let result = crate::pty_custody::replace_exact_record_observed_with_fence(
        &guard,
        &path,
        expected,
        expected_identity,
        replacement,
        |identity| committed.set(Some(identity)),
        |_| {
            reconcile::require_runtime_exact(
                guard.namespace(),
                &runtime_path,
                runtime,
                runtime_identity,
            )
        },
    )
    .and_then(|installed| {
        installed_identity.set(Some(installed));
        ensure!(
            installed.same_content_version(committed.get().context(RUNTIME_IDENTITY_DETAIL)?),
            RUNTIME_IDENTITY_DETAIL
        );
        after_install()?;
        reconcile::require_runtime_exact(
            guard.namespace(),
            &runtime_path,
            runtime,
            runtime_identity,
        )?;
        namespace.verify()?;
        ensure!(
            exact_identity(namespace, &path, replacement)? == installed,
            RUNTIME_IDENTITY_DETAIL
        );
        Ok(installed)
    });
    match result {
        Ok(installed) => Ok(installed),
        Err(original) => reconcile::committed(
            namespace,
            &path,
            replacement,
            committed.get(),
            installed_identity.get(),
            &runtime_path,
            runtime,
            runtime_identity,
        )
        .map_err(|reconcile| original.context(reconcile)),
    }
}

fn exact_identity(
    namespace: &WorkerNamespace,
    path: &std::path::Path,
    expected: &CustodyDurableRecord,
) -> Result<FileIdentity> {
    let (current, identity): (CustodyDurableRecord, _) = namespace.read_json_with_identity(path)?;
    ensure!(current == *expected, RUNTIME_IDENTITY_DETAIL);
    Ok(identity)
}

fn validate_custody(runtime: &RuntimeRecord, custody: &CustodyDurableRecord) -> Result<()> {
    ensure!(
        custody.is_valid() && custody.runtime.matches_record(runtime),
        RUNTIME_IDENTITY_DETAIL
    );
    Ok(())
}

fn validate_manifest(
    namespace: &WorkerNamespace,
    paths: &Paths,
    runtime: &RuntimeRecord,
) -> Result<()> {
    let manifest: crate::model::TerminalManifest = namespace
        .read_json(&paths.manifest(runtime.terminal_id))
        .context(MANIFEST_CONTEXT)?;
    ensure!(
        manifest.schema_version == crate::constants::SESSION_MANIFEST_SCHEMA_VERSION
            && manifest.terminal_id == runtime.terminal_id
            && !manifest.archived,
        MANIFEST_DETAIL
    );
    Ok(())
}

pub(crate) const RUNTIME_PUBLICATION_LOCK_CONTEXT: &str = "session lifecycle mutation is in progress";
pub(crate) const RUNTIME_PUBLICATION_REJECTED: &str =
    "runtime publication requires exact durable process authority";

pub(crate) fn publish_runtime_record(
    namespace: &WorkerNamespace,
    paths: &Paths,
    record: &RuntimeRecord,
) -> Result<()> {
    let guard = publication_guard(namespace, paths, record.terminal_id)?;
    publish_runtime_record_guarded(namespace, paths, record, &guard)
}

pub(crate) fn publish_runtime_record_guarded(
    namespace: &WorkerNamespace,
    paths: &Paths,
    record: &RuntimeRecord,
    guard: &yolo_paths::durable_mutation::SessionDurableMutationGuard<'_>,
) -> Result<()> {
    let manifest = runtime_publication_fence::ManifestFence::capture(namespace, paths, record)
        .context(RUNTIME_PUBLICATION_REJECTED)?;
    let path = paths.runtime_record(record.terminal_id);
    anyhow::ensure!(
        !namespace.file_is_missing(&path)?,
        RUNTIME_PUBLICATION_REJECTED
    );
    let (current, expected): (RuntimeRecord, _) = namespace.read_json_with_identity(&path)?;
    record
        .validate_update_from(&current)
        .context(RUNTIME_PUBLICATION_REJECTED)?;
    if current == *record {
        guard.require_no_pending_intent()?;
        manifest.require_exact(namespace, paths, record)?;
        let confirmed: (RuntimeRecord, _) = namespace.read_json_with_identity(&path)?;
        anyhow::ensure!(
            confirmed == (current, expected),
            RUNTIME_PUBLICATION_REJECTED
        );
        guard.require_no_pending_intent()?;
        return record_runtime_publication(record);
    }
    guard.write_json_expected_with_fence(&path, Some(expected), record, |_| {
        manifest.require_exact(namespace, paths, record)
    })?;
    record_runtime_publication(record)
}

include!("runtime_initial_publication.rs");

pub(crate) fn publication_guard<'a>(
    namespace: &'a WorkerNamespace,
    paths: &Paths,
    terminal_id: Uuid,
) -> Result<yolo_paths::durable_mutation::SessionDurableMutationGuard<'a>> {
    yolo_paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
        paths,
        namespace.lease(),
        terminal_id,
    )
    .context(RUNTIME_PUBLICATION_LOCK_CONTEXT)
}

pub(crate) fn record_runtime_publication(_record: &RuntimeRecord) -> Result<()> {
    #[cfg(test)]
    runtime_persistence_test_hook::record_publication(_record)?;
    Ok(())
}

#[cfg(test)]
pub(crate) fn persist_bootstrap_record<T: serde::Serialize>(
    namespace: &WorkerNamespace,
    path: &Path,
    value: &T,
) -> Result<()> {
    persist_bootstrap_record_with(namespace, path, value, || Ok(()))
}

#[cfg(test)]
pub(crate) async fn persist_pending_then_release<P, R, F, T>(persist: P, release: R) -> Result<T>
where
    P: FnOnce() -> Result<()>,
    R: FnOnce() -> F,
    F: Future<Output = Result<T>>,
{
    persist()?;
    release().await
}

#[cfg(test)]
pub(crate) fn persist_bootstrap_record_with<T, F>(
    namespace: &WorkerNamespace,
    path: &Path,
    value: &T,
    after_write: F,
) -> Result<()>
where
    T: serde::Serialize,
    F: FnOnce() -> Result<()>,
{
    namespace.write_json(path, value)?;
    after_write()?;
    namespace.verify()
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-bootstrap"))]
#[path = "../../unit_tests/bootstrap/bootstrap_persistence_tests.rs"]
mod bootstrap_persistence_tests;

pub(crate) fn publish_initial_runtime_record(
    namespace: &WorkerNamespace,
    paths: &Paths,
    record: &RuntimeRecord,
    checkpoint: &startup_checkpoint::ActiveStartupCheckpoint,
    predecessor: Option<&startup_checkpoint::StartupPredecessorAuthority>,
) -> Result<()> {
    publish_initial_runtime_record_with(
        namespace,
        paths,
        record,
        checkpoint,
        predecessor,
        || Ok(()),
    )
}

#[cfg(test)]
pub(crate) fn publish_initial_runtime_record_with_test_hook<F>(
    namespace: &WorkerNamespace,
    paths: &Paths,
    record: &RuntimeRecord,
    checkpoint: &startup_checkpoint::ActiveStartupCheckpoint,
    predecessor: Option<&startup_checkpoint::StartupPredecessorAuthority>,
    before_runtime_cas: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    publish_initial_runtime_record_with(
        namespace,
        paths,
        record,
        checkpoint,
        predecessor,
        before_runtime_cas,
    )
}

pub(crate) fn publish_initial_runtime_record_with<F>(
    namespace: &WorkerNamespace,
    paths: &Paths,
    record: &RuntimeRecord,
    checkpoint: &startup_checkpoint::ActiveStartupCheckpoint,
    predecessor: Option<&startup_checkpoint::StartupPredecessorAuthority>,
    before_runtime_cas: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    let guard = publication_guard(namespace, paths, record.terminal_id)?;
    let fence = runtime_publication_fence::InitialPublicationFence::capture(
        namespace, paths, record, checkpoint,
    )?;
    let expected = initial_runtime_identity(namespace, paths, record, predecessor)?;
    before_runtime_cas()?;
    let path = paths.runtime_record(record.terminal_id);
    guard.write_json_expected_with_fence(&path, expected, record, |phase| {
        fence.require_exact(namespace, paths, record, checkpoint)?;
        if phase == yolo_paths::durable_mutation::DurableMutationFencePhase::Commit {
            require_exact_predecessor(namespace, paths, record, predecessor, expected)?;
        }
        Ok(())
    })?;
    record_runtime_publication(record)
}

pub(crate) fn initial_runtime_identity(
    namespace: &WorkerNamespace,
    paths: &Paths,
    record: &RuntimeRecord,
    predecessor: Option<&startup_checkpoint::StartupPredecessorAuthority>,
) -> Result<Option<crate::paths::identity::FileIdentity>> {
    if let Some(authority) = predecessor {
        return startup_checkpoint::validate_predecessor_publication(
            namespace, paths, record, authority,
        )
        .map(Some);
    }
    anyhow::ensure!(
        namespace.file_is_missing(&paths.runtime_record(record.terminal_id))?,
        RUNTIME_PUBLICATION_REJECTED
    );
    Ok(None)
}

pub(crate) fn require_exact_predecessor(
    namespace: &WorkerNamespace,
    paths: &Paths,
    record: &RuntimeRecord,
    predecessor: Option<&startup_checkpoint::StartupPredecessorAuthority>,
    expected: Option<crate::paths::identity::FileIdentity>,
) -> Result<()> {
    let Some(authority) = predecessor else {
        return Ok(());
    };
    anyhow::ensure!(
        Some(startup_checkpoint::validate_predecessor_publication(
            namespace, paths, record, authority,
        )?) == expected,
        RUNTIME_PUBLICATION_REJECTED
    );
    Ok(())
}

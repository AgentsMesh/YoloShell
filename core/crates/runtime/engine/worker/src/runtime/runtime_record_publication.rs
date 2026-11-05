#[cfg(feature = "startup-handoff-test-hooks")]
use super::publish_runtime_record;
#[cfg(feature = "startup-handoff-test-hooks")]
use super::startup_handoff_test_hook;
use super::{
    StartupPtyResources, WorkerNamespace, publish_initial_runtime_record, startup_checkpoint,
};
#[cfg(feature = "startup-handoff-test-hooks")]
use crate::model::BootstrapDelivery;
use crate::model::RuntimeRecord;
use crate::paths::Paths;
use anyhow::Result;
#[cfg(feature = "startup-handoff-test-hooks")]
use anyhow::{Context, bail};
use portable_pty::MasterPty;

pub(crate) async fn publish_pending(
    namespace: &WorkerNamespace,
    paths: &Paths,
    record: RuntimeRecord,
    _startup: &mut StartupPtyResources<Box<dyn MasterPty + Send>>,
    _checkpoint: &mut startup_checkpoint::ActiveStartupCheckpoint,
) -> Result<RuntimeRecord> {
    #[cfg(feature = "startup-handoff-test-hooks")]
    let mut record = record;
    #[cfg(feature = "startup-handoff-test-hooks")]
    if startup_handoff_test_hook::requested()? {
        publish_initial(namespace, paths, &record, _checkpoint)?;
        _checkpoint.complete(
            namespace,
            paths,
            startup_checkpoint::StartupStage::RuntimeRecord,
        )?;
        startup_handoff_test_hook::publish_ready(&record)?;
        record.bootstrap_delivery = BootstrapDelivery::Failed;
        publish_runtime_record(namespace, paths, &record)?;
        startup_handoff_test_hook::prepare_failure(&record, _startup.child_mut())?;
        startup_handoff_test_hook::wait_for_release().await?;
        namespace
            .remove_file(&paths.worker_launch(record.terminal_id))
            .context(super::WORKER_LAUNCH_RETIRE_CONTEXT)?;
        bail!(startup_handoff_test_hook::INJECTED_FAILURE_DETAIL);
    }
    publish_initial(namespace, paths, &record, _checkpoint)?;
    Ok(record)
}

pub(crate) fn publish_initial(
    namespace: &WorkerNamespace,
    paths: &Paths,
    record: &RuntimeRecord,
    checkpoint: &mut startup_checkpoint::ActiveStartupCheckpoint,
) -> Result<()> {
    publish_initial_runtime_record(
        namespace,
        paths,
        record,
        checkpoint,
        checkpoint.predecessor_authority(),
    )?;
    checkpoint.commit_runtime_predecessor_publication();
    Ok(())
}

#[cfg(test)]
pub(crate) fn publish_initial_with_test_hook<F>(
    namespace: &WorkerNamespace,
    paths: &Paths,
    record: &RuntimeRecord,
    checkpoint: &mut startup_checkpoint::ActiveStartupCheckpoint,
    before_runtime_cas: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    super::publish_initial_runtime_record_with_test_hook(
        namespace,
        paths,
        record,
        checkpoint,
        checkpoint.predecessor_authority(),
        before_runtime_cas,
    )?;
    checkpoint.commit_runtime_predecessor_publication();
    Ok(())
}

use crate::process::linux_containment::{
    LinuxContainmentBroker, LinuxContainmentLease, record_path,
};
pub(crate) const PREPARE_CONTEXT: &str = "prepare exclusive Linux session cgroup";
pub(crate) const PERSIST_CONTEXT: &str = "persist Linux session cgroup identity";

pub(crate) fn prepare_startup_containment(
    namespace: &WorkerNamespace,
    paths: &Paths,
    launch: &WorkerLaunchSpec,
) -> Result<Option<LinuxContainmentLease>> {
    let Some(broker) = LinuxContainmentBroker::discover().context(PREPARE_CONTEXT)? else {
        return Ok(None);
    };
    let lease = broker
        .prepare(launch.terminal_id, launch.runtime_epoch)
        .context(PREPARE_CONTEXT)?;
    let persisted = persist_startup_containment(namespace, paths, lease.record());
    if let Err(error) = persisted {
        return Err(match lease.remove_empty() {
            Ok(()) => error,
            Err(cleanup) => error.context(cleanup.to_string()),
        });
    }
    Ok(Some(lease))
}

pub(crate) fn persist_startup_containment(
    namespace: &WorkerNamespace,
    paths: &Paths,
    record: &crate::process::linux_containment::LinuxContainmentRecord,
) -> Result<()> {
    namespace
        .write_json(
            &record_path(&paths.session_dir(record.terminal_id), record.runtime_epoch),
            record,
        )
        .and_then(|()| namespace.verify())
        .context(PERSIST_CONTEXT)
}

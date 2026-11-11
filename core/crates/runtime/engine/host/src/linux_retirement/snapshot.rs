use crate::host::{Host, RuntimeRecord};
use crate::paths::identity::FileIdentity;
use crate::process::{ProcessIdentity, linux_containment::LinuxContainmentRecord};
use anyhow::{Result, ensure};

const CONTAINMENT_DETAIL: &str = "Linux retirement containment identity changed";

pub(super) struct ContainmentSnapshot {
    pub(super) record: LinuxContainmentRecord,
    identity: FileIdentity,
}

impl ContainmentSnapshot {
    pub(super) fn capture(host: &Host, runtime: &RuntimeRecord) -> Result<Self> {
        let path = containment_path(host, runtime);
        let (record, identity): (LinuxContainmentRecord, FileIdentity) =
            host.namespace.read_json_with_identity(&path)?;
        ensure!(
            record.validate_recovery(
                runtime.terminal_id,
                runtime.runtime_epoch,
                &host.boot_id,
                root(runtime),
            )?,
            CONTAINMENT_DETAIL
        );
        let snapshot = Self { record, identity };
        snapshot.verify(host, runtime)?;
        Ok(snapshot)
    }

    pub(super) fn verify(&self, host: &Host, runtime: &RuntimeRecord) -> Result<()> {
        let current: (LinuxContainmentRecord, FileIdentity) = host
            .namespace
            .read_json_with_identity(&containment_path(host, runtime))?;
        ensure!(
            current == (self.record.clone(), self.identity),
            CONTAINMENT_DETAIL
        );
        ensure!(
            self.record.validate_recovery(
                runtime.terminal_id,
                runtime.runtime_epoch,
                &host.boot_id,
                root(runtime),
            )?,
            CONTAINMENT_DETAIL
        );
        Ok(())
    }
}

pub(super) fn root(runtime: &RuntimeRecord) -> ProcessIdentity {
    ProcessIdentity {
        pid: runtime.child_pid,
        start_time: runtime.child_start_time,
    }
}

fn containment_path(host: &Host, runtime: &RuntimeRecord) -> std::path::PathBuf {
    crate::process::linux_containment::record_path(
        &host.paths.session_dir(runtime.terminal_id),
        runtime.runtime_epoch,
    )
}

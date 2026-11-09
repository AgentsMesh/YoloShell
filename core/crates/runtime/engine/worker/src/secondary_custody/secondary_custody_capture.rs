use super::{StartupPtyResources, WorkerSecondaryCustodySource};
use crate::model::RuntimeRecord;
#[cfg(target_os = "linux")]
use anyhow::Context;
use anyhow::Result;
use portable_pty::MasterPty;
use std::sync::Arc;

#[cfg(target_os = "linux")]
const MASTER_DESCRIPTOR_CONTEXT: &str = "startup PTY master has no transferable descriptor";

pub(crate) fn capture(
    startup: &mut StartupPtyResources<Box<dyn MasterPty + Send>>,
    record: &RuntimeRecord,
) -> Result<Arc<WorkerSecondaryCustodySource>> {
    #[cfg(target_os = "macos")]
    let platform = startup.child().export_secondary_custody(
        &crate::pty_custody::CustodyRuntimeIdentity::from_record(record),
    )?;
    #[cfg(target_os = "linux")]
    let platform = {
        let descriptor = startup
            .master()?
            .as_raw_fd()
            .context(MASTER_DESCRIPTOR_CONTEXT)?;
        startup.child().export_secondary_custody(descriptor)?
    };
    let transfer = Arc::new(super::startup_custody_transfer::StartupCustodyTransferFence::new());
    startup
        .child_mut()
        .bind_custody_transfer_fence(Arc::clone(&transfer))?;
    Ok(Arc::new(WorkerSecondaryCustodySource::new(
        platform, transfer,
    )))
}

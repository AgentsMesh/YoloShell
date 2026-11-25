use crate::process::{LiveProcessIdentity, MacosProcessCapabilityManagerReader, ProcessIdentity};
use anyhow::{Context, Result};

const ROOT_LIVENESS_CONTEXT: &str = "arm transferred macOS custody root liveness";

pub(super) fn arm_transferred_liveness(
    reader: &MacosProcessCapabilityManagerReader,
    root: ProcessIdentity,
) -> Result<()> {
    let Some(live) = LiveProcessIdentity::capture(root)? else {
        return Ok(());
    };
    match reader.arm_liveness(live.clone()) {
        Ok(()) => Ok(()),
        Err(error) if live.try_matches()? => Err(error).context(ROOT_LIVENESS_CONTEXT),
        Err(_) => Ok(()),
    }
}

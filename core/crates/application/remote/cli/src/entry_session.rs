use crate::cli::Command;
use anyhow::Result;

pub(super) fn initialize(command: &Command) -> Result<()> {
    if matches!(command, Command::BrokerSupervisor(_)) {
        yolo_process::establish_detached_session()?;
    }
    Ok(())
}

use super::{CtlCommand, CtlOptions};
use anyhow::Result;
use std::io::Write;
use yolo_runtime::host::{Host, HostAbsence};

pub(super) fn absence(options: &CtlOptions) -> Result<Option<HostAbsence>> {
    if !options.existing_host
        || options.existing_socket
        || !matches!(
            options.command,
            CtlCommand::Status | CtlCommand::List { .. }
        )
    {
        return Ok(None);
    }
    let home = options.home.as_ref().expect("clap requires local home");
    Host::observe_absence(home, options.socket.as_deref())
}

pub(super) fn write_absence(absence: HostAbsence, mut output: impl Write) -> Result<()> {
    absence.verify()?;
    output.write_all(&yolo_capnp_codec::encode(&())?)?;
    output.flush()?;
    absence.verify()
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/ctl_presence.rs"]
mod tests;

use anyhow::{Result, bail};
use std::ffi::OsString;
use yolo_launch_contract::{Invocation, ProcessRole};

const INVALID_FIXTURE_ROLE: &str = "bootstrap fixture only accepts the explicit Hub role";

pub fn dispatch(
    arguments: impl IntoIterator<Item = OsString>,
    tui: impl FnOnce(Vec<OsString>) -> Result<()>,
    hub: impl FnOnce(Vec<OsString>) -> Result<()>,
) -> Result<()> {
    let invocation = Invocation::parse(arguments)?;
    match invocation.role {
        None => tui(invocation.arguments),
        Some(ProcessRole::Hub) => hub(invocation.arguments),
        Some(_) => bail!(INVALID_FIXTURE_ROLE),
    }
}

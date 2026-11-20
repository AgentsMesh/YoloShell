use anyhow::Result;
use portable_pty::{Child, CommandBuilder, SlavePty};

pub(crate) fn spawn_waitable_pty_root(
    slave: &dyn SlavePty,
    command: CommandBuilder,
) -> Result<Box<dyn Child + Send + Sync>> {
    let _guard = crate::waitable_child_spawn_guard()?;
    slave.spawn_command(command)
}

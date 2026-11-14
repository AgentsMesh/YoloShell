use super::command::{DetachedStandardError, spawn_detached_with_stdio};
use anyhow::Result;
use std::process::{Child, Command, Stdio};

pub fn spawn_detached_with_diagnostic_pipes(command: Command) -> Result<Child> {
    spawn_detached_with_stdio(
        command,
        DetachedStandardError::Piped,
        Stdio::piped(),
        Stdio::piped(),
        None,
    )
}

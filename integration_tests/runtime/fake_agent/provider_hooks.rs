use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::fake_agent_constants::{
    PACKAGED_SETTINGS_COMMAND_FIELD as COMMAND, PACKAGED_SETTINGS_HOOKS_FIELD as HOOKS,
};
const TYPE: &str = "type";
const MATCHER: &str = "matcher";
const ARGUMENTS: &str = "args";
const SHELL: &str = "/bin/sh";
const SHELL_COMMAND: &str = "-c";
const INVALID: &str = "fake provider hooks configuration is invalid";
const STALLED_INPUT: &str = "fake provider hook stdin is unavailable";
const FAILED: &str = "fake provider hook command failed";
const MAX_BYTES: u64 = 1024 * 1024;
const READ_SENTINEL: u64 = 1;
const MATCH_ALL: &[&str] = &["", "*"];
const RESTRICTIONS: &[&str] = &["disableAllHooks", "allowManagedHooksOnly"];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HookCommand {
    pub(crate) command: String,
}

#[derive(Default)]
pub(crate) struct Settings {
    pub(crate) hooks: BTreeMap<String, Vec<HookCommand>>,
}

pub(crate) fn load(path: &Path) -> Result<Settings, Box<dyn Error>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Settings::default());
        }
        Err(error) => return Err(error.into()),
    };
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + READ_SENTINEL)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(INVALID.into());
    }
    parse(&serde_json::from_slice(&bytes)?)
}

fn parse(root: &Value) -> Result<Settings, Box<dyn Error>> {
    let root = root.as_object().ok_or(INVALID)?;
    for key in RESTRICTIONS {
        if let Some(value) = root.get(*key) {
            if value.as_bool().ok_or(INVALID)? {
                return Ok(Settings::default());
            }
        }
    }
    let Some(hooks) = root.get(HOOKS) else {
        return Ok(Settings::default());
    };
    let mut settings = Settings::default();
    for (event, groups) in hooks.as_object().ok_or(INVALID)? {
        let commands = settings.hooks.entry(event.clone()).or_default();
        for group in groups.as_array().ok_or(INVALID)? {
            let group = group.as_object().ok_or(INVALID)?;
            if group.get(MATCHER).is_some_and(|value| {
                !value
                    .as_str()
                    .is_some_and(|matcher| MATCH_ALL.contains(&matcher))
            }) {
                continue;
            }
            for spec in group.get(HOOKS).and_then(Value::as_array).ok_or(INVALID)? {
                let spec = spec.as_object().ok_or(INVALID)?;
                if spec.get(TYPE).and_then(Value::as_str) != Some(COMMAND)
                    || spec.contains_key(ARGUMENTS)
                {
                    return Err(INVALID.into());
                }
                let command = spec
                    .get(COMMAND)
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
                    .ok_or(INVALID)?;
                commands.push(HookCommand {
                    command: command.to_owned(),
                });
            }
        }
    }
    Ok(settings)
}

pub(crate) fn execute(
    hook: &HookCommand,
    input: &impl serde::Serialize,
) -> Result<(), Box<dyn Error>> {
    run_command(shell_command(hook), input)
}

fn shell_command(hook: &HookCommand) -> Command {
    let mut command = Command::new(SHELL);
    command.args([SHELL_COMMAND, &hook.command]);
    command
}

fn run_command(mut command: Command, input: &impl serde::Serialize) -> Result<(), Box<dyn Error>> {
    let encoded = serde_json::to_vec(input)?;
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()?;
    let mut stdin = child.stdin.take().ok_or(STALLED_INPUT)?;
    let written = stdin.write_all(&encoded).and_then(|()| stdin.flush());
    drop(stdin);
    if !child.wait()?.success() {
        return Err(FAILED.into());
    }
    if let Err(error) = written {
        if error.kind() != std::io::ErrorKind::BrokenPipe {
            return Err(error.into());
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "unit_tests/installed_hooks.rs"]
mod installed_tests;
#[cfg(test)]
#[path = "unit_tests/provider_hooks.rs"]
mod tests;

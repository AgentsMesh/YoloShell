use crate::{
    claude_provider_input,
    fake_agent_constants::*,
    packaged_claude_transcript::Transcript,
    provider_hooks::{self, Settings},
};
use std::{
    error::Error,
    io::Write,
    path::{Component, PathBuf},
};

const COMMAND_SEPARATOR: char = ' ';
const INVALID_DIRECTORY: &str = "fake Claude directory command must name an absolute directory";
const MISSING_DIRECTORY_HOOK: &str = "fake Claude directory hook is not installed";
const MAX_DIRECTORY_COMMAND_BYTES: usize = 16 * 1024;

pub(crate) fn handle(
    line: &str,
    settings: &Settings,
    transcript: &mut Transcript,
    session_id: &str,
) -> Result<bool, Box<dyn Error>> {
    let Some(requested) = directory(line)? else {
        return Ok(false);
    };
    let hooks = settings
        .hooks
        .get(yolo_runtime::claude_hooks::CLAUDE_CWD_CHANGED_EVENT)
        .filter(|hooks| !hooks.is_empty())
        .ok_or(MISSING_DIRECTORY_HOOK)?;
    let old_cwd = std::env::current_dir()?.canonicalize()?;
    let requested = requested.canonicalize()?;
    std::env::set_current_dir(&requested)?;
    let new_cwd = std::env::current_dir()?.canonicalize()?;
    transcript.waiting(session_id, &new_cwd)?;
    let input =
        claude_provider_input::cwd_changed(session_id, transcript.path(), &old_cwd, &new_cwd)?;
    for hook in hooks {
        provider_hooks::execute(hook, &input)?;
    }
    println!("{PACKAGED_TUI_CWD_ACK}");
    std::io::stdout().flush()?;
    Ok(true)
}

fn directory(line: &str) -> Result<Option<PathBuf>, Box<dyn Error>> {
    let Some((command, value)) = line.split_once(COMMAND_SEPARATOR) else {
        return Ok(None);
    };
    if command != PACKAGED_TUI_CWD_COMMAND {
        return Ok(None);
    }
    if value.len() > MAX_DIRECTORY_COMMAND_BYTES {
        return Err(INVALID_DIRECTORY.into());
    }
    let value: String = serde_json::from_str(value)?;
    let path = PathBuf::from(value);
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(INVALID_DIRECTORY.into());
    }
    Ok(Some(path))
}

#[cfg(test)]
#[path = "unit_tests/claude_directory.rs"]
mod tests;

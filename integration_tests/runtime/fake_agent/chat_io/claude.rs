use crate::{
    fake_agent_constants::*, packaged_claude::deliver, packaged_claude_settings::Settings,
};
use std::{error::Error, path::Path};

pub fn prompt(
    settings: &Settings,
    session: &str,
    transcript: &Path,
    cwd: &Path,
) -> Result<(), Box<dyn Error>> {
    deliver(
        settings,
        PACKAGED_USER_PROMPT_EVENT,
        session,
        transcript,
        cwd,
        None,
        None,
    )
}

pub fn ready(
    settings: &Settings,
    session: &str,
    transcript: &Path,
    cwd: &Path,
) -> Result<(), Box<dyn Error>> {
    deliver(
        settings,
        PACKAGED_STOP_EVENT,
        session,
        transcript,
        cwd,
        None,
        None,
    )?;
    // Claude's real idle_prompt Notification is separate evidence that its
    // editor accepts the next prompt. Stop itself conveys no such authority.
    deliver(
        settings,
        PACKAGED_NOTIFICATION_EVENT,
        session,
        transcript,
        cwd,
        None,
        Some(PACKAGED_WAITING_NOTIFICATION_TYPE),
    )
}

use crate::packaged_claude_settings::HookCommand;
use std::error::Error;
use std::path::Path;

pub(crate) fn deliver(
    hook: &HookCommand,
    event: &str,
    session_id: &str,
    transcript: &Path,
    cwd: &Path,
    source: Option<&str>,
    notification_type: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let input = crate::claude_provider_input::build(
        event,
        session_id,
        transcript,
        cwd,
        source,
        notification_type,
    )?;
    crate::provider_hooks::execute(hook, &input)
}

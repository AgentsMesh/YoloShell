use crate::{fake_agent_constants as constants, provider::FakeAgentProvider, provider_hooks};
use std::error::Error;
use std::path::Path;
use yolo_runtime::codex_hooks::CodexHookInput;

const CONFIGURATION_FILE: &str = "hooks.json";

pub(crate) fn version_output(provider: FakeAgentProvider, arguments: &[String]) -> Option<String> {
    (provider == FakeAgentProvider::Codex && arguments == [constants::CODEX_VERSION_ARGUMENT]).then(
        || {
            format!(
                "{}{}\n",
                constants::CODEX_VERSION_OUTPUT_PREFIX,
                provider_version_text()
            )
        },
    )
}

pub(crate) fn deliver_session_start(
    provider: FakeAgentProvider,
    home: &Path,
    arguments: &[String],
    session_id: &str,
    transcript_path: &Path,
    cwd: &Path,
) -> Result<(), Box<dyn Error>> {
    if provider != FakeAgentProvider::Codex {
        return Ok(());
    }
    let source =
        session_start_source(arguments).ok_or(constants::CODEX_SESSION_START_ARGUMENTS_INVALID)?;
    let payload = session_start_payload(session_id, transcript_path, cwd, source);
    let settings = provider_hooks::load(&home.join(CONFIGURATION_FILE))?;
    if let Some(commands) = settings.hooks.get(constants::CODEX_SESSION_START_EVENT) {
        for command in commands {
            provider_hooks::execute(command, &payload)?;
        }
    }
    Ok(())
}

fn session_start_source(arguments: &[String]) -> Option<&'static str> {
    match arguments {
        [] => Some(constants::CODEX_SESSION_STARTUP_SOURCE),
        [resume, cwd_argument, _, _]
            if resume == yolo_runtime::agent::codex::constants::RESUME_ARGUMENT
                && cwd_argument == yolo_runtime::agent::codex::constants::CWD_ARGUMENT =>
        {
            Some(constants::CODEX_SESSION_RESUME_SOURCE)
        }
        _ => None,
    }
}

fn session_start_payload(
    session_id: &str,
    transcript_path: &Path,
    cwd: &Path,
    source: &str,
) -> CodexHookInput {
    CodexHookInput::SessionStart {
        session_id: session_id.to_owned(),
        transcript_path: Some(transcript_path.to_path_buf()),
        cwd: cwd.to_path_buf(),
        model: constants::CODEX_MODEL.to_owned(),
        permission_mode: constants::CODEX_PERMISSION_DEFAULT.to_owned(),
        source: source.to_owned(),
    }
}

fn provider_version_text() -> String {
    let version = yolo_runtime::codex_hooks::ACCEPTED_PROVIDER_VERSION;
    format!(
        "{}{}{}{}{}",
        version.major,
        constants::CODEX_VERSION_SEPARATOR,
        version.minor,
        constants::CODEX_VERSION_SEPARATOR,
        version.patch
    )
}

#[cfg(test)]
pub(crate) fn session_start_payload_for_test(
    session_id: &str,
    transcript_path: &Path,
    cwd: &Path,
    source: &str,
) -> serde_json::Value {
    serde_json::to_value(session_start_payload(
        session_id,
        transcript_path,
        cwd,
        source,
    ))
    .unwrap()
}

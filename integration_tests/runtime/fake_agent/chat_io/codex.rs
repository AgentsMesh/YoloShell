use super::constants::*;
use crate::{fake_agent_constants, provider_hooks};
use std::{error::Error, fs::OpenOptions, io::Write, path::Path};
use yolo_runtime::codex_hooks::CodexHookInput;

pub(crate) fn run(home: &Path, transcript: &Path, session: &str) -> Result<(), Box<dyn Error>> {
    let settings = provider_hooks::load(&home.join(CODEX_CONFIG))?;
    // main delivered the single real SessionStart. Stop is only emitted after
    // processing a submitted turn and does not prove that a new prompt is ready.
    super::run_editor(|text| {
        let turn_id = uuid::Uuid::new_v4().to_string();
        prompt(&settings, transcript, session, &turn_id, text)?;
        let mut file = OpenOptions::new().append(true).open(transcript)?;
        for (role, kind, text) in [(USER, INPUT_TEXT, text), (ASSISTANT, OUTPUT_TEXT, ANSWER)] {
            let record = serde_json::json!({(TYPE): RESPONSE_ITEM, (PAYLOAD): {
                (TYPE): MESSAGE, (ROLE): role, (CONTENT): [{(TYPE): kind, (TEXT): text}],
            }});
            serde_json::to_writer(&mut file, &record)?;
            file.write_all(b"\n")?;
        }
        file.flush()?;
        file.sync_data()?;
        stop(&settings, transcript, session, &turn_id)
    })
}

fn stop(
    settings: &provider_hooks::Settings,
    transcript: &Path,
    session: &str,
    turn_id: &str,
) -> Result<(), Box<dyn Error>> {
    let input = CodexHookInput::Stop {
        session_id: session.into(),
        transcript_path: Some(transcript.to_path_buf()),
        cwd: std::env::current_dir()?.canonicalize()?,
        model: fake_agent_constants::CODEX_MODEL.into(),
        permission_mode: fake_agent_constants::CODEX_PERMISSION_DEFAULT.into(),
        turn_id: turn_id.into(),
        stop_hook_active: false,
        last_assistant_message: Some(ANSWER.into()),
    };
    if let Some(hooks) = settings.hooks.get(STOP_EVENT) {
        for hook in hooks {
            provider_hooks::execute(hook, &input)?;
        }
    }
    Ok(())
}

fn prompt(
    settings: &provider_hooks::Settings,
    transcript: &Path,
    session: &str,
    turn_id: &str,
    text: &str,
) -> Result<(), Box<dyn Error>> {
    let input = CodexHookInput::UserPromptSubmit {
        session_id: session.into(),
        transcript_path: Some(transcript.to_path_buf()),
        cwd: std::env::current_dir()?.canonicalize()?,
        model: fake_agent_constants::CODEX_MODEL.into(),
        permission_mode: fake_agent_constants::CODEX_PERMISSION_DEFAULT.into(),
        turn_id: turn_id.into(),
        prompt: text.into(),
        agent_id: None,
        agent_type: None,
    };
    if let Some(hooks) = settings.hooks.get(USER_PROMPT_EVENT) {
        for hook in hooks {
            provider_hooks::execute(hook, &input)?;
        }
    }
    Ok(())
}

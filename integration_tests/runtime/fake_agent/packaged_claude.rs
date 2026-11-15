use crate::fake_agent_constants::*;
use crate::packaged_claude_hook;
use crate::packaged_claude_invocation;
use crate::packaged_claude_record::{self, InvocationRecord};
use crate::packaged_claude_settings::{self, Settings};
use crate::packaged_claude_transcript::Transcript;
use std::error::Error;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
#[path = "packaged_claude_scenario.rs"]
mod scenario;
use scenario::{Step, configured_scenario};

const DATA_HOME_ERROR: &str = "packaged Claude data home is unavailable";
const MARKER_ERROR: &str = "packaged Claude launch marker is incomplete";

pub(crate) fn run(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let packaged = std::env::var(PACKAGED_E2E_ENV).ok();
    let scenario = configured_scenario(packaged.as_deref());
    let invocation = packaged_claude_invocation::parse(arguments)?;
    let session_id = &invocation.session_id;
    let cwd = std::env::current_dir()?.canonicalize()?;
    let data_home = data_home()?;
    let settings = packaged_claude_settings::load(&data_home)?;
    let mut transcript = Transcript::open(&data_home, &session_id, &cwd)?;
    for step in scenario.steps() {
        apply_step(
            *step,
            &settings,
            &mut transcript,
            session_id,
            &cwd,
            invocation.mode.source(),
        )?;
    }
    record_invocation(InvocationRecord {
        mode: invocation.mode.label(),
        session_id,
        arguments,
        hook_count: settings.hooks.values().map(Vec::len).sum(),
        cwd: &cwd,
        data_home: &data_home,
        transcript: transcript.path(),
    })?;
    launch_marker()?;
    tui_loop(&settings, &mut transcript, session_id)
}

fn apply_step(
    step: Step,
    settings: &Settings,
    transcript: &mut Transcript,
    session_id: &str,
    cwd: &Path,
    source: &str,
) -> Result<(), Box<dyn Error>> {
    match step {
        Step::SessionStart => deliver(
            settings,
            PACKAGED_SESSION_START_EVENT,
            session_id,
            transcript.path(),
            cwd,
            Some(source),
            None,
        ),
        Step::TranscriptRunning => transcript.running(session_id, cwd),
        Step::UserPrompt => deliver(
            settings,
            PACKAGED_USER_PROMPT_EVENT,
            session_id,
            transcript.path(),
            cwd,
            None,
            None,
        ),
        Step::TranscriptWaiting => transcript.waiting(session_id, cwd),
        Step::Stop => deliver(
            settings,
            PACKAGED_STOP_EVENT,
            session_id,
            transcript.path(),
            cwd,
            None,
            None,
        ),
        Step::WaitingNotification => deliver(
            settings,
            PACKAGED_NOTIFICATION_EVENT,
            session_id,
            transcript.path(),
            cwd,
            None,
            Some(PACKAGED_WAITING_NOTIFICATION_TYPE),
        ),
    }
}

fn record_invocation(record: InvocationRecord<'_>) -> Result<(), Box<dyn Error>> {
    if std::env::var_os(PACKAGED_RECORD_DIRECTORY_ENV).is_some() {
        return packaged_claude_record::write(record);
    }
    let runtime_epoch = std::env::var(yolo_runtime::agent::constants::YOLO_RUNTIME_EPOCH_ENV)?;
    crate::record_invocation(record.data_home, &runtime_epoch, record.arguments)?;
    Ok(())
}

fn data_home() -> Result<PathBuf, Box<dyn Error>> {
    let path = std::env::var_os(yolo_runtime::agent::claude::constants::DATA_HOME_ENV)
        .map(PathBuf::from)
        .ok_or(DATA_HOME_ERROR)?;
    if !path.is_absolute() {
        return Err(DATA_HOME_ERROR.into());
    }
    std::fs::create_dir_all(&path)?;
    Ok(path.canonicalize()?)
}

pub(crate) fn deliver(
    settings: &Settings,
    event: &str,
    session_id: &str,
    transcript: &Path,
    cwd: &Path,
    source: Option<&str>,
    notification_type: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    if let Some(commands) = settings.hooks.get(event) {
        for hook in commands {
            packaged_claude_hook::deliver(
                hook,
                event,
                session_id,
                transcript,
                cwd,
                source,
                notification_type,
            )?;
        }
    }
    Ok(())
}

fn launch_marker() -> Result<(), Box<dyn Error>> {
    let values = [
        std::env::var(PACKAGED_MARKER_LEFT_ENV).ok(),
        std::env::var(PACKAGED_MARKER_RIGHT_ENV).ok(),
        std::env::var(PACKAGED_MARKER_NONCE_ENV).ok(),
    ];
    if values.iter().all(Option::is_none) {
        return Ok(());
    }
    let [Some(left), Some(right), Some(nonce)] = values else {
        return Err(MARKER_ERROR.into());
    };
    println!("{left}{right}{nonce}");
    std::io::stdout().flush()?;
    Ok(())
}

fn tui_loop(
    settings: &Settings,
    transcript: &mut Transcript,
    session_id: &str,
) -> Result<(), Box<dyn Error>> {
    if crate::chat_io::configured() {
        return crate::chat_io::run(settings, transcript, session_id);
    }
    for line in std::io::stdin().lock().lines() {
        let line = line?;
        if crate::packaged_claude_directory::handle(&line, settings, transcript, session_id)? {
            continue;
        }
        if let Some(marker) = ping_marker(&line) {
            println!("{marker}");
            std::io::stdout().flush()?;
        }
    }
    Ok(())
}

fn ping_marker(line: &str) -> Option<String> {
    let fields = line.split_whitespace().collect::<Vec<_>>();
    match fields.as_slice() {
        [command, left, right, nonce] if *command == PACKAGED_TUI_PING_COMMAND => {
            Some(format!("{left}{right}{nonce}"))
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "packaged_claude_mode_tests.rs"]
mod tests;

mod chat_io;
mod claude_provider_input;
mod claude_scenario;
mod claude_scenario_process;
mod claude_transport;
mod claude_wire;
#[cfg(test)]
mod claude_wire_path;
mod codex_cli;
#[path = "constants.rs"]
mod fake_agent_constants;
mod invocation;
mod native_control_observer;
mod packaged_claude;
mod packaged_claude_directory;
mod packaged_claude_hook;
mod packaged_claude_invocation;
mod packaged_claude_record;
mod packaged_claude_settings;
mod packaged_claude_transcript;
mod packaged_codex_tui;
mod provider;
mod provider_hooks;

#[cfg(test)]
mod claude_protocol_name_tests;
#[cfg(test)]
mod claude_scenario_tests;
#[cfg(test)]
#[path = "unit_tests/claude_settings.rs"]
mod claude_settings_tests;
#[cfg(test)]
mod claude_wire_tests;
#[cfg(test)]
mod codex_cli_tests;
#[cfg(test)]
mod invocation_tests;
#[cfg(test)]
mod packaged_claude_hook_tests;
#[cfg(test)]
mod packaged_claude_record_tests;
#[cfg(test)]
#[path = "test_support/packaged_claude.rs"]
mod packaged_claude_test_support;
#[cfg(test)]
mod packaged_claude_transcript_tests;
#[cfg(test)]
mod packaged_codex_tui_tests;
#[cfg(test)]
mod tests;

use provider::FakeAgentProvider;
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, ErrorKind, Write};
use std::path::Path;
use yolo_runtime::agent::constants as agent;

const PROCESS_ARGUMENT_PREFIX_COUNT: usize = 1;
const RESUME_SESSION_ID_MISSING: &str = "resume session id is missing";
const RESUME_CWD_MISMATCH: &str = "resume cwd does not match the process cwd";
const INVOCATION_ARGUMENTS_UNSUPPORTED: &str = "fake agent invocation arguments are unsupported";
const INVOCATION_WRITE_INCOMPLETE: &str = "fake agent invocation append was incomplete";

fn main() -> Result<(), Box<dyn Error>> {
    let provider = FakeAgentProvider::from_process()?;
    let raw_arguments = std::env::args()
        .skip(PROCESS_ARGUMENT_PREFIX_COUNT)
        .collect::<Vec<_>>();
    if let Some(output) = codex_cli::version_output(provider, &raw_arguments) {
        print!("{output}");
        return Ok(());
    }
    let _signal_observer = native_control_observer::install_if_configured()?;
    let arguments = raw_arguments;
    if provider == FakeAgentProvider::Claude
        && claude_scenario_process::run_if_configured(&arguments)?
    {
        return Ok(());
    }
    if provider == FakeAgentProvider::Claude {
        return packaged_claude::run(&arguments);
    }
    let runtime_epoch = std::env::var(agent::YOLO_RUNTIME_EPOCH_ENV)?;
    let session_id = session_id(provider, &arguments, &runtime_epoch)?;
    let data_home = std::env::var(provider.data_home_environment())?;
    let data_home = Path::new(&data_home);
    fs::create_dir_all(data_home)?;
    let data_home = data_home.canonicalize()?;
    let sessions = data_home.join(provider.evidence_directory());
    fs::create_dir_all(&sessions)?;
    record_invocation(&data_home, &runtime_epoch, &arguments)?;
    let evidence_path = provider.evidence_path(&sessions, session_id);
    let _evidence_writer = write_evidence(provider, &sessions, session_id)?;
    codex_cli::deliver_session_start(
        provider,
        &data_home,
        &arguments,
        session_id,
        &evidence_path,
        &std::env::current_dir()?,
    )?;
    if provider == FakeAgentProvider::Codex {
        if chat_io::configured() {
            return chat_io::codex::run(&data_home, &evidence_path, session_id);
        }
        return packaged_codex_tui::run();
    }
    loop {
        std::thread::park();
    }
}

fn session_id<'a>(
    provider: FakeAgentProvider,
    arguments: &'a [String],
    runtime_epoch: &'a str,
) -> Result<&'a str, &'static str> {
    match arguments {
        [] => Ok(runtime_epoch),
        [argument, cwd_argument, cwd, session_id]
            if provider == FakeAgentProvider::Codex
                && argument == provider.resume_argument()
                && cwd_argument == yolo_runtime::agent::codex::constants::CWD_ARGUMENT =>
        {
            if Path::new(cwd) != std::env::current_dir().map_err(|_| RESUME_CWD_MISMATCH)? {
                return Err(RESUME_CWD_MISMATCH);
            }
            Ok(session_id.as_str())
        }
        [argument, session_id]
            if provider == FakeAgentProvider::Claude && argument == provider.resume_argument() =>
        {
            Ok(session_id.as_str())
        }
        [argument] if argument == provider.resume_argument() => Err(RESUME_SESSION_ID_MISSING),
        _ => Err(INVOCATION_ARGUMENTS_UNSUPPORTED),
    }
}

fn record_invocation(
    data_home: &Path,
    runtime_epoch: &str,
    arguments: &[String],
) -> std::io::Result<()> {
    let encoded = invocation::encode(runtime_epoch, arguments).map_err(std::io::Error::other)?;
    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(data_home.join(fake_agent_constants::INVOCATION_LOG_FILE))?;
    let written = log.write(&encoded)?;
    if written == encoded.len() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            ErrorKind::WriteZero,
            INVOCATION_WRITE_INCOMPLETE,
        ))
    }
}

fn write_evidence(
    provider: FakeAgentProvider,
    directory: &Path,
    session_id: &str,
) -> Result<BufWriter<File>, Box<dyn Error>> {
    let path = provider.evidence_path(directory, session_id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)?;
    let mut writer = BufWriter::new(file);
    for record in provider.initial_evidence(session_id, &std::env::current_dir()?) {
        serde_json::to_writer(&mut writer, &record)?;
        writeln!(writer)?;
    }
    writer.flush()?;
    Ok(writer)
}

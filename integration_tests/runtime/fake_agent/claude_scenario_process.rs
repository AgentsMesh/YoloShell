use crate::claude_scenario::{
    ClaudeScenario, HOOK_SOCKET_ENV, HookAvailability, SCENARIO_FILE_ENV, execute,
};
use crate::claude_transport::JsonLineHookAdapter;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub(crate) const MAX_SCENARIO_BYTES: u64 = 1_048_576;
const SCENARIO_TOO_LARGE: &str = "fake Claude scenario exceeds its size limit";
const HOOK_SOCKET_MISSING: &str = "fake Claude hook socket is not configured";
const INVOCATION_MISMATCH: &str = "fake Claude scenario does not match its invocation";
const SIZE_LIMIT_PROBE_BYTES: u64 = 1;

pub(crate) fn run_if_configured(arguments: &[String]) -> Result<bool, Box<dyn Error>> {
    let Some(path) = std::env::var_os(SCENARIO_FILE_ENV) else {
        return Ok(false);
    };
    let scenario = load(Path::new(&path))?;
    match scenario.hook_availability {
        HookAvailability::Enabled => {
            validate_invocation(&scenario, arguments)?;
            let socket = std::env::var_os(HOOK_SOCKET_ENV).ok_or(HOOK_SOCKET_MISSING)?;
            let mut adapter = JsonLineHookAdapter::connect(Path::new(&socket))?;
            execute(&scenario, &mut adapter)?;
        }
        HookAvailability::Missing | HookAvailability::Error | HookAvailability::DisableAllHooks => {
            execute(&scenario, &mut NoopAdapter)?;
        }
    }
    Ok(true)
}

fn validate_invocation(
    scenario: &ClaudeScenario,
    arguments: &[String],
) -> Result<(), Box<dyn Error>> {
    use crate::claude_wire::{ClaudeHookInput, SessionStartSource};
    let Some(ClaudeHookInput::SessionStart { common, source }) =
        scenario.frames.first().map(|frame| &frame.input)
    else {
        return Err(INVOCATION_MISMATCH.into());
    };
    let valid = match arguments {
        [] => *source == SessionStartSource::Startup,
        [resume, session_id]
            if resume == yolo_runtime::agent::claude::constants::RESUME_ARGUMENT =>
        {
            *source == SessionStartSource::Resume && common.session_id == *session_id
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(INVOCATION_MISMATCH.into())
    }
}

pub(crate) fn load(path: &Path) -> Result<ClaudeScenario, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut bytes = Vec::new();
    file.take(MAX_SCENARIO_BYTES + SIZE_LIMIT_PROBE_BYTES)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_SCENARIO_BYTES {
        return Err(SCENARIO_TOO_LARGE.into());
    }
    Ok(serde_json::from_slice(&bytes)?)
}

struct NoopAdapter;

impl crate::claude_scenario::ClaudeHookAdapter for NoopAdapter {
    fn deliver(
        &mut self,
        _frame: &crate::claude_wire::ClaudeHookFrame,
    ) -> crate::claude_scenario::AdapterResult {
        Ok(())
    }
}

use crate::fake_agent_constants::*;
use std::error::Error;

const INVOCATION_ERROR: &str = "packaged Claude invocation arguments are invalid";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Mode {
    Fresh,
    Resume,
}

impl Mode {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Fresh => PACKAGED_FRESH_MODE,
            Self::Resume => PACKAGED_RESUME_MODE,
        }
    }
    pub(crate) fn source(self) -> &'static str {
        match self {
            Self::Fresh => PACKAGED_START_SOURCE,
            Self::Resume => PACKAGED_RESUME_SOURCE,
        }
    }
}

pub(crate) struct Invocation {
    pub(crate) mode: Mode,
    pub(crate) session_id: String,
}

pub(crate) fn parse(arguments: &[String]) -> Result<Invocation, Box<dyn Error>> {
    let (mode, session_id) = match arguments {
        [] => (Mode::Fresh, uuid::Uuid::new_v4().to_string()),
        [resume_argument, session_id]
            if resume_argument == yolo_runtime::agent::claude::constants::RESUME_ARGUMENT =>
        {
            (Mode::Resume, session_id.clone())
        }
        _ => return Err(INVOCATION_ERROR.into()),
    };
    let parsed = uuid::Uuid::parse_str(&session_id).map_err(|_| INVOCATION_ERROR)?;
    if parsed.is_nil() || parsed.hyphenated().to_string() != session_id {
        return Err(INVOCATION_ERROR.into());
    }
    Ok(Invocation { mode, session_id })
}

#[cfg(test)]
#[path = "unit_tests/claude_invocation.rs"]
mod tests;

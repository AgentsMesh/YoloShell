//! `--current` inherits the caller terminal's Hub and workspace from the
//! environment Yolo injected into the managed shell. The values are selectors,
//! not authority: the Hub still validates the workspace and every control
//! precondition.

use crate::constants::{CURRENT_HUB_MISMATCH, CURRENT_WORKSPACE_INVALID};
use crate::reject::rejected;
use anyhow::Result;
use uuid::Uuid;
use yolo_agent_adapter_contract::constants::{
    YOLO_HUB_ID_ENV, YOLO_RUN_ID_ENV, YOLO_WORKSPACE_ID_ENV,
};

/// Resolve the workspace for a new terminal. `--current` requires a managed
/// caller terminal and inherits its injected workspace id when present.
pub fn workspace(current: bool, explicit: Option<Uuid>) -> Result<Option<Uuid>> {
    if !current {
        return Ok(explicit);
    }
    crate::current::terminal_selector()?;
    parse_workspace(std::env::var(YOLO_WORKSPACE_ID_ENV).ok().as_deref())
}

/// Reject an inherited Hub id that does not match the Hub this client reached.
pub fn hub(current: bool, connected: Uuid) -> Result<()> {
    if !current {
        return Ok(());
    }
    check_hub(
        std::env::var(YOLO_HUB_ID_ENV).ok().as_deref(),
        connected,
    )
}

/// The run that owns the caller terminal. A shell started by `agent start`
/// receives this identity; a start from it becomes a child unless the caller
/// names a parent explicitly.
pub fn run() -> Option<String> {
    std::env::var(YOLO_RUN_ID_ENV)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

pub(crate) fn parse_workspace(value: Option<&str>) -> Result<Option<Uuid>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    match Uuid::parse_str(value) {
        Ok(workspace) if !workspace.is_nil() => Ok(Some(workspace)),
        _ => Err(rejected(CURRENT_WORKSPACE_INVALID)),
    }
}

pub(crate) fn check_hub(value: Option<&str>, connected: Uuid) -> Result<()> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    match Uuid::parse_str(value) {
        Ok(inherited) if inherited == connected => Ok(()),
        _ => Err(rejected(CURRENT_HUB_MISMATCH)),
    }
}

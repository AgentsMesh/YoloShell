//! Hub-owned caller context applied to the PTY child environment. These
//! variables come from the Host creation identity, never from client argv.

use portable_pty::CommandBuilder;
use uuid::Uuid;

pub fn apply(
    command: &mut CommandBuilder,
    hub_id: Uuid,
    workspace_id: Option<Uuid>,
    run_id: Option<Uuid>,
) {
    command.env(
        yolo_agent_core::constants::YOLO_HUB_ID_ENV,
        hub_id.to_string(),
    );
    if let Some(workspace_id) = workspace_id {
        command.env(
            yolo_agent_core::constants::YOLO_WORKSPACE_ID_ENV,
            workspace_id.to_string(),
        );
    }
    if let Some(run_id) = run_id {
        command.env(
            yolo_agent_core::constants::YOLO_RUN_ID_ENV,
            run_id.to_string(),
        );
    }
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

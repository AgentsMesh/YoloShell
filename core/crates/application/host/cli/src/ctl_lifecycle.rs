use crate::LifecyclePreconditions;
use anyhow::{Context, Result};
use yolo_client::Client;
use yolo_runtime::model::{TerminalLifecycleSelection, TerminalRuntimeStatus, TerminalView};

const LIFECYCLE_STATUS_INVALID: &str = "invalid expected session status";

pub(super) async fn archive(
    client: &Client,
    terminal_id: uuid::Uuid,
    preconditions: LifecyclePreconditions,
) -> Result<TerminalView> {
    let selection = lifecycle_selection(terminal_id, preconditions)?;
    Ok(client.archive_with_preconditions(selection).await?)
}

pub(super) async fn terminate(
    client: &Client,
    terminal_id: uuid::Uuid,
    force: bool,
    preconditions: LifecyclePreconditions,
) -> Result<()> {
    let selection = lifecycle_selection(terminal_id, preconditions)?;
    client
        .terminate_with_preconditions(selection, force)
        .await
        .map_err(Into::into)
}

pub(super) async fn discard(
    client: &Client,
    terminal_id: uuid::Uuid,
    force: bool,
    preconditions: LifecyclePreconditions,
) -> Result<TerminalView> {
    let selection = lifecycle_selection(terminal_id, preconditions)?;
    Ok(client.discard_with_preconditions(selection, force).await?)
}

pub(super) async fn resume(
    client: &Client,
    terminal_id: uuid::Uuid,
    recreate: bool,
    preconditions: LifecyclePreconditions,
) -> Result<TerminalView> {
    let selection = lifecycle_selection(terminal_id, preconditions)?;
    Ok(client
        .resume_with_preconditions(selection, recreate)
        .await?)
}

fn lifecycle_selection(
    terminal_id: uuid::Uuid,
    preconditions: LifecyclePreconditions,
) -> Result<TerminalLifecycleSelection> {
    let selection = match (
        preconditions.expected_updated_at_ms,
        preconditions.expected_status,
    ) {
        (Some(updated), Some(status)) => {
            TerminalLifecycleSelection::selected(terminal_id, updated, lifecycle_status(&status)?)
        }
        _ => TerminalLifecycleSelection::unconditional(terminal_id),
    };
    Ok(selection
        .with_expected_manifest_sha256(preconditions.expected_manifest_sha256)
        .with_expected_runtime_epoch(preconditions.expected_runtime_epoch))
}

fn lifecycle_status(value: &str) -> Result<TerminalRuntimeStatus> {
    yolo_capnp_codec::from_value(yolo_capnp_codec::Value::Text(value.into()))
        .context(LIFECYCLE_STATUS_INVALID)
}

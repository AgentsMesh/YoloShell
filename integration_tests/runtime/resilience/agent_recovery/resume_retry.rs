use super::*;

pub(super) async fn resume_until_complete(
    harness: &Harness,
    terminal_id: Uuid,
    force: bool,
) -> Result<TerminalView, ClientError> {
    let deadline = tokio::time::Instant::now() + AGENT_STATE_SETTLEMENT_TIMEOUT;
    loop {
        match harness.resume_and_track(terminal_id, force).await {
            Ok(view) => return Ok(view),
            Err(error) => {
                if let Some(view) = completed_view(harness, terminal_id).await {
                    return Ok(view);
                }
                if !retryable(&error) || tokio::time::Instant::now() >= deadline {
                    return Err(error);
                }
                tokio::time::sleep(POLL_INTERVAL).await;
            }
        }
    }
}

async fn completed_view(harness: &Harness, terminal_id: Uuid) -> Option<TerminalView> {
    let view = harness.client().get_session(terminal_id).await.ok()?;
    let complete = view.manifest.recovery_state == RecoveryState::Resumed
        && view
            .runtime
            .as_ref()
            .is_some_and(|runtime| runtime.status == yolo_runtime::TerminalRuntimeStatus::Running);
    complete.then(|| {
        harness.track_view(&view);
        view
    })
}

fn retryable(error: &ClientError) -> bool {
    matches!(
        error,
        ClientError::Remote {
            retryable: true,
            ..
        }
    )
}

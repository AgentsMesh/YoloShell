use super::native_client::NativeClient;
use crate::claude_hooks::{ClaudeLifecycleEvent, native_outbox};
use crate::timing::{HOOK_DELIVERY_BUDGET, HOOK_RETRY_DELAY};
use anyhow::Result;

pub(super) async fn deliver(event: ClaudeLifecycleEvent) -> Result<()> {
    let Some(client) = NativeClient::from_environment()? else {
        return Ok(());
    };
    deliver_with(&client, event, HOOK_DELIVERY_BUDGET).await
}

async fn deliver_with(
    client: &NativeClient,
    event: ClaudeLifecycleEvent,
    budget: std::time::Duration,
) -> Result<()> {
    native_outbox::enqueue(&client.outbox, client.scope, event)?;
    let deadline = tokio::time::Instant::now() + budget;
    while let Some(request) = native_outbox::next(&client.outbox, client.scope)? {
        loop {
            if tokio::time::Instant::now() >= deadline {
                return Ok(());
            }
            if let Ok(Ok(response)) = tokio::time::timeout_at(deadline, client.send(&request)).await
            {
                if response.acknowledges(&request) {
                    native_outbox::acknowledge(&client.outbox, &request)?;
                    break;
                }
            }
            tokio::time::sleep_until(
                (tokio::time::Instant::now() + HOOK_RETRY_DELAY).min(deadline),
            )
            .await;
        }
    }
    Ok(())
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/native_delivery_deadline_tests.rs"]
mod deadline_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/native_delivery_tests.rs"]
mod tests;

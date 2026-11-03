use super::cancellation::IngressCancellation;
use super::constants::{
    CODEX_INGRESS_ACCEPT_RETRY_ERROR, CODEX_INGRESS_ACCEPT_RETRY_LIMIT,
    CODEX_INGRESS_CONNECTION_DRAIN_ERROR, CODEX_INGRESS_CONNECTION_DRAIN_TIMEOUT,
    CODEX_INGRESS_CONNECTION_ERROR,
};
use super::server::CodexHookServer;
use crate::worker::supervision;
use anyhow::{Context, Result};
use futures::FutureExt;
use std::panic::AssertUnwindSafe;
use tokio::task::JoinSet;

#[path = "ingress_task_supervision.rs"]
mod task_supervision;
pub(super) use task_supervision::run;
#[cfg(test)]
pub(super) use task_supervision::{run_with_tasks_for_test, settle_intake};

pub(super) async fn serve(
    server: &mut CodexHookServer,
    cancellation: &IngressCancellation,
    connections: &mut JoinSet<()>,
) -> Result<()> {
    let mut accept_failures = 0_u32;
    loop {
        tokio::select! {
            biased;
            _ = cancellation.cancelled() => break,
            completed = connections.join_next(), if !connections.is_empty() => {
                contain_connection_completion(completed);
            }
            accepted = server.accept() => match accepted {
                Ok((stream, _)) => {
                    accept_failures = 0;
                    let Some(permit) = server.try_admit_client() else {
                        continue;
                    };
                    let router = server.router();
                    let transaction = server.transaction();
                    let settlements = server.settlements();
                    let generation = server.generation();
                    connections.spawn(handle_connection(
                        stream,
                        router,
                        transaction,
                        permit,
                        settlements,
                        generation,
                    ));
                }
                Err(error) => {
                    if !repair_listener(
                        server,
                        cancellation,
                        &mut accept_failures,
                        error.into(),
                    ).await? {
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

async fn repair_listener(
    server: &mut CodexHookServer,
    cancellation: &IngressCancellation,
    failure_count: &mut u32,
    mut error: anyhow::Error,
) -> Result<bool> {
    loop {
        let delay = listener_retry(error, failure_count)?;
        tokio::select! {
            biased;
            _ = cancellation.cancelled() => return Ok(false),
            _ = tokio::time::sleep(delay) => {}
        }
        match server.restart_retained().await {
            Ok(()) => {
                *failure_count = 0;
                return Ok(true);
            }
            Err(next_error) => error = next_error,
        }
    }
}

fn listener_retry(error: anyhow::Error, failure_count: &mut u32) -> Result<std::time::Duration> {
    *failure_count = failure_count.saturating_add(1);
    if *failure_count > CODEX_INGRESS_ACCEPT_RETRY_LIMIT {
        return Err(error).context(CODEX_INGRESS_ACCEPT_RETRY_ERROR);
    }
    let failure = supervision::TaskFailure::message(
        supervision::ACCEPT_TASK,
        supervision::error_ref_detail(&error),
    );
    Ok(supervision::report_restart(
        &failure,
        supervision::RestartPolicy::default(),
        *failure_count,
    ))
}

pub(super) async fn drain_connections(connections: &mut JoinSet<()>) {
    if tokio::time::timeout(
        CODEX_INGRESS_CONNECTION_DRAIN_TIMEOUT,
        drain_pending(connections),
    )
    .await
    .is_ok()
    {
        return;
    }
    tracing::warn!("{CODEX_INGRESS_CONNECTION_DRAIN_ERROR}");
    connections.abort_all();
    drain_pending(connections).await;
}

async fn drain_pending(connections: &mut JoinSet<()>) {
    while let Some(completed) = connections.join_next().await {
        contain_connection_completion(Some(completed));
    }
}

async fn handle_connection(
    stream: crate::local_unix::LocalUnixStream,
    router: std::sync::Arc<super::router::CodexSessionRouter>,
    transaction: std::sync::Arc<super::hook_context::CodexHookContext>,
    permit: tokio::sync::OwnedSemaphorePermit,
    settlements: super::blocking_custody::BlockingSettlementCustody,
    generation: super::cancellation::IngressCancellation,
) {
    let result = AssertUnwindSafe(async move {
        let _permit = permit;
        super::routed_handler::handle(stream, router, transaction, settlements, generation).await
    })
    .catch_unwind()
    .await;
    match result {
        Ok(Ok(outcome)) => {
            let (_, response) = outcome.into_parts();
            if let Err(error) = response {
                tracing::debug!(detail = %error, "{CODEX_INGRESS_CONNECTION_ERROR}");
            }
        }
        Ok(Err(error)) => tracing::debug!(detail = %error, "{CODEX_INGRESS_CONNECTION_ERROR}"),
        Err(payload) => tracing::error!(
            detail = %supervision::panic_detail(payload),
            "{CODEX_INGRESS_CONNECTION_ERROR}"
        ),
    }
}

fn contain_connection_completion(
    completed: Option<std::result::Result<(), tokio::task::JoinError>>,
) {
    if let Some(Err(error)) = completed {
        if !error.is_cancelled() {
            tracing::error!(detail = %error, "{CODEX_INGRESS_CONNECTION_ERROR}");
        }
    }
}

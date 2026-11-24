use super::{CodexHookServer, IngressCancellation};
use crate::worker::supervision;
use anyhow::{Context, Result};
use futures::FutureExt;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use tokio::task::JoinSet;

pub(in crate::worker::codex_hook) async fn run(
    server: CodexHookServer,
    cancellation: IngressCancellation,
) -> Result<()> {
    let intake =
        super::super::intake_task::run(server.router(), cancellation.clone(), server.settlements());
    run_supervised(server, cancellation, intake, JoinSet::new()).await
}

#[cfg(test)]
pub(in crate::worker::codex_hook) async fn run_with_tasks_for_test<Intake>(
    server: CodexHookServer,
    cancellation: IngressCancellation,
    intake: Intake,
    connections: JoinSet<()>,
) -> Result<()>
where
    Intake: Future<Output = Result<()>> + Send + 'static,
{
    run_supervised(server, cancellation, intake, connections).await
}

async fn run_supervised<Intake>(
    mut server: CodexHookServer,
    cancellation: IngressCancellation,
    intake: Intake,
    mut connections: JoinSet<()>,
) -> Result<()>
where
    Intake: Future<Output = Result<()>> + Send + 'static,
{
    let settlements = server.settlements();
    let mut intake_task = JoinSet::new();
    intake_task.spawn(intake);
    let result = {
        let serving = AssertUnwindSafe(super::serve(&mut server, &cancellation, &mut connections))
            .catch_unwind();
        tokio::pin!(serving);
        tokio::select! {
            biased;
            completed = intake_task.join_next() => {
                let primary = expected_completion(
                    settle_intake(completed),
                    &cancellation,
                    super::super::constants::CODEX_INTAKE_TASK_ERROR,
                );
                cancellation.cancel();
                primary.and(settle_serve(serving.await))
            }
            completed = &mut serving => {
                let primary = expected_completion(
                    settle_serve(completed),
                    &cancellation,
                    super::super::constants::CODEX_INGRESS_TASK_ERROR,
                );
                cancellation.cancel();
                primary.and(settle_intake(intake_task.join_next().await))
            }
        }
    };
    super::drain_connections(&mut connections).await;
    let blocking_settlement = tokio::time::timeout(
        super::super::constants::CODEX_INGRESS_BLOCKING_DRAIN_TIMEOUT,
        settlements.wait_idle(),
    )
    .await
    .context(super::super::constants::CODEX_INGRESS_BLOCKING_DRAIN_ERROR);
    let final_intake = if result.is_ok() && blocking_settlement.is_ok() {
        super::super::intake_task::reconcile(server.router(), settlements).await
    } else {
        Ok(())
    };
    result.and(blocking_settlement).and(final_intake)
}

fn expected_completion(
    completed: Result<()>,
    cancellation: &IngressCancellation,
    early_completion: &'static str,
) -> Result<()> {
    match completed {
        Ok(()) if !cancellation.is_cancelled() => anyhow::bail!(early_completion),
        result => result,
    }
}

pub(in crate::worker::codex_hook) fn settle_intake(
    completed: Option<std::result::Result<Result<()>, tokio::task::JoinError>>,
) -> Result<()> {
    let completed = completed
        .context(super::super::constants::CODEX_INTAKE_TASK_ERROR)?
        .context(super::super::constants::CODEX_INTAKE_TASK_ERROR)?;
    completed.context(super::super::constants::CODEX_INTAKE_TASK_ERROR)
}

fn settle_serve(completed: std::thread::Result<Result<()>>) -> Result<()> {
    match completed {
        Ok(result) => result,
        Err(payload) => {
            let detail = supervision::panic_detail(payload);
            Err(anyhow::anyhow!(detail).context(super::super::constants::CODEX_INGRESS_TASK_ERROR))
        }
    }
}

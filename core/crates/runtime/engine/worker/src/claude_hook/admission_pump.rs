use super::ClaudeHookAdmissionPump;
use super::constants::HOOK_ADMISSION_TASK;
use super::server::ClaudeHookServer;
use crate::worker::{WorkerState, supervision};
use anyhow::{Context, Result};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;
use tokio::sync::oneshot;
use tokio::task::JoinSet;

const PUMP_READY_RECEIVER_DROPPED: &str = "Claude hook admission readiness receiver was dropped";
const PUMP_LISTENER_ARM_FAILED: &str = "Claude hook admission listener failed before readiness";
const PUMP_ACTIVATION_DROPPED: &str = "Claude hook runtime activation sender was dropped";
#[cfg(test)]
const TEST_ARM_OBSERVER_DROPPED: &str = "Claude hook admission test arm observer was dropped";
#[cfg(test)]
const TEST_READY_RELEASE_DROPPED: &str = "Claude hook admission test readiness release was dropped";

pub(in crate::worker) fn spawn_admission_pump(
    tasks: &mut JoinSet<Option<supervision::TaskFailure>>,
    server: ClaudeHookServer,
    state: Arc<WorkerState>,
    shell: crate::process::ProcessIdentity,
) -> ClaudeHookAdmissionPump {
    spawn_after(tasks, server, state, shell, std::future::ready(Ok(())))
}

#[cfg(test)]
pub(in crate::worker) fn spawn_admission_pump_after_test_barrier(
    tasks: &mut JoinSet<Option<supervision::TaskFailure>>,
    server: ClaudeHookServer,
    state: Arc<WorkerState>,
    shell: crate::process::ProcessIdentity,
    armed: oneshot::Sender<()>,
    release: oneshot::Receiver<()>,
) -> ClaudeHookAdmissionPump {
    spawn_after(tasks, server, state, shell, async move {
        armed
            .send(())
            .map_err(|_| anyhow::anyhow!(TEST_ARM_OBSERVER_DROPPED))?;
        release.await.context(TEST_READY_RELEASE_DROPPED)
    })
}

fn spawn_after<F>(
    tasks: &mut JoinSet<Option<supervision::TaskFailure>>,
    server: ClaudeHookServer,
    state: Arc<WorkerState>,
    shell: crate::process::ProcessIdentity,
    readiness_barrier: F,
) -> ClaudeHookAdmissionPump
where
    F: Future<Output = Result<()>> + Send + 'static,
{
    let (ready_sender, ready) = oneshot::channel();
    let (activation, activated) = oneshot::channel();
    supervision::spawn_task(tasks, HOOK_ADMISSION_TASK, async move {
        run(
            server,
            state,
            shell,
            ready_sender,
            activated,
            readiness_barrier,
        )
        .await
    });
    ClaudeHookAdmissionPump::new(ready, activation)
}

async fn run<F>(
    mut server: ClaudeHookServer,
    state: Arc<WorkerState>,
    shell: crate::process::ProcessIdentity,
    ready: oneshot::Sender<()>,
    mut activated: oneshot::Receiver<()>,
    readiness_barrier: F,
) -> Result<()>
where
    F: Future<Output = Result<()>>,
{
    let mut connections = JoinSet::new();
    let mut retry: Option<Pin<Box<tokio::time::Sleep>>> = None;
    let mut failures = 0_u32;
    let pending_connection = poll_accept_once(&server)
        .await
        .transpose()
        .context(PUMP_LISTENER_ARM_FAILED)?;
    readiness_barrier.await?;
    ready
        .send(())
        .map_err(|_| anyhow::anyhow!(PUMP_READY_RECEIVER_DROPPED))?;
    let mut runtime_active = false;
    if let Some(accepted) = pending_connection {
        super::handle_accept_result(
            Ok(accepted),
            &mut connections,
            &state,
            &server,
            shell,
            runtime_active,
            &mut failures,
        );
    }
    loop {
        tokio::select! {
            activation = &mut activated, if !runtime_active => {
                activation.context(PUMP_ACTIVATION_DROPPED)?;
                runtime_active = true;
            }
            accepted = server.accept(), if retry.is_none() => {
                retry = super::handle_accept_result(
                    accepted,
                    &mut connections,
                    &state,
                    &server,
                    shell,
                    runtime_active,
                    &mut failures,
                ).map(|delay| Box::pin(tokio::time::sleep(delay)));
            }
            _ = wait_for_retry(&mut retry) => {
                retry = None;
                if let Err(error) = server.restart(&state.namespace).await {
                    let delay = super::handle_listener_failure(error, &mut failures);
                    retry = Some(Box::pin(tokio::time::sleep(delay)));
                }
            }
            completed = connections.join_next(), if !connections.is_empty() => {
                report_connection_completion(completed);
            }
        }
    }
}

async fn poll_accept_once(
    server: &ClaudeHookServer,
) -> Option<
    std::io::Result<(
        crate::local_unix::LocalUnixStream,
        tokio::net::unix::SocketAddr,
    )>,
> {
    let mut accepted = Box::pin(server.accept());
    std::future::poll_fn(|context| match accepted.as_mut().poll(context) {
        Poll::Ready(result) => Poll::Ready(Some(result)),
        Poll::Pending => Poll::Ready(None),
    })
    .await
}

async fn wait_for_retry(retry: &mut Option<Pin<Box<tokio::time::Sleep>>>) {
    match retry {
        Some(delay) => delay.await,
        None => std::future::pending().await,
    }
}

fn report_connection_completion(
    completed: Option<
        std::result::Result<Option<supervision::TaskFailure>, tokio::task::JoinError>,
    >,
) {
    let failure = match completed {
        Some(Ok(Some(failure))) => Some(failure),
        Some(Err(error)) => Some(supervision::TaskFailure::message(
            HOOK_ADMISSION_TASK,
            error.to_string(),
        )),
        _ => None,
    };
    if let Some(failure) = failure {
        supervision::report_contained(&failure);
    }
}

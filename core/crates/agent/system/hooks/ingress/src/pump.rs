use super::constants::HOOK_CONNECTION_TASK;
use super::handler;
use super::host::Context;
use super::server::HookIngressServer;
use anyhow::{Context as _, Result};
use std::future::Future;
use std::pin::Pin;
use std::task::Poll;
use tokio::sync::oneshot;
use tokio::task::JoinSet;

const PUMP_READY_RECEIVER_DROPPED: &str = "provider hook admission readiness receiver was dropped";
const PUMP_LISTENER_ARM_FAILED: &str = "provider hook admission listener failed before readiness";
const PUMP_ACTIVATION_DROPPED: &str = "provider hook runtime activation sender was dropped";

pub struct HookIngressPump {
    readiness: Option<oneshot::Receiver<()>>,
    activation: Option<oneshot::Sender<()>>,
}

impl HookIngressPump {
    pub async fn wait_ready(&mut self) -> Result<()> {
        self.readiness
            .take()
            .context(PUMP_READY_RECEIVER_DROPPED)?
            .await
            .context(PUMP_READY_RECEIVER_DROPPED)
    }

    pub fn take_readiness(&mut self) -> Result<oneshot::Receiver<()>> {
        self.readiness.take().context(PUMP_READY_RECEIVER_DROPPED)
    }

    pub fn activate(mut self) -> Result<()> {
        self.activation
            .take()
            .context(PUMP_ACTIVATION_DROPPED)?
            .send(())
            .map_err(|_| anyhow::anyhow!(PUMP_ACTIVATION_DROPPED))
    }
}

pub fn start(
    server: HookIngressServer,
    context: Context,
) -> (HookIngressPump, impl Future<Output = Result<()>>) {
    let (ready_sender, ready) = oneshot::channel();
    let (activation, activated) = oneshot::channel();
    let pump = HookIngressPump {
        readiness: Some(ready),
        activation: Some(activation),
    };
    (pump, run(server, context, ready_sender, activated))
}

async fn run(
    mut server: HookIngressServer,
    context: Context,
    ready: oneshot::Sender<()>,
    mut activated: oneshot::Receiver<()>,
) -> Result<()> {
    let mut connections = JoinSet::new();
    let mut retry: Option<Pin<Box<tokio::time::Sleep>>> = None;
    let mut failures = 0_u32;
    let pending_connection = poll_accept_once(&server)
        .await
        .transpose()
        .context(PUMP_LISTENER_ARM_FAILED)?;
    ready
        .send(())
        .map_err(|_| anyhow::anyhow!(PUMP_READY_RECEIVER_DROPPED))?;
    let mut runtime_active = false;
    if let Some(accepted) = pending_connection {
        retry = handle_accept_result(
            Ok(accepted),
            &mut connections,
            &server,
            &context,
            runtime_active,
            &mut failures,
        )
        .map(|delay| Box::pin(tokio::time::sleep(delay)));
    }
    loop {
        tokio::select! {
            activation = &mut activated, if !runtime_active => {
                activation.context(PUMP_ACTIVATION_DROPPED)?;
                runtime_active = true;
            }
            accepted = server.accept(), if retry.is_none() => {
                retry = handle_accept_result(
                    accepted,
                    &mut connections,
                    &server,
                    &context,
                    runtime_active,
                    &mut failures,
                )
                .map(|delay| Box::pin(tokio::time::sleep(delay)));
            }
            _ = wait_for_retry(&mut retry) => {
                retry = None;
                if let Err(error) = server.restart(&context.host.namespace).await {
                    let delay = handle_listener_failure(error);
                    retry = Some(Box::pin(tokio::time::sleep(delay)));
                }
            }
            completed = connections.join_next(), if !connections.is_empty() => {
                if let Some(Err(error)) = completed {
                    tracing::warn!(detail = %error, "provider hook connection task failed");
                }
            }
        }
    }
}

async fn poll_accept_once(
    server: &HookIngressServer,
) -> Option<std::io::Result<(yolo_local_unix::LocalUnixStream, tokio::net::unix::SocketAddr)>> {
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

fn handle_accept_result(
    accepted: std::io::Result<(yolo_local_unix::LocalUnixStream, tokio::net::unix::SocketAddr)>,
    tasks: &mut JoinSet<Result<()>>,
    server: &HookIngressServer,
    context: &Context,
    runtime_active: bool,
    failure_count: &mut u32,
) -> Option<std::time::Duration> {
    match accepted {
        Ok((stream, _)) => {
            *failure_count = 0;
            let permit = server.admit_client()?;
            let context = Context {
                host: context.host.clone(),
                executable: std::sync::Arc::clone(&context.executable),
            };
            tasks.spawn(async move {
                let _permit = permit;
                handler::handle(stream, context, runtime_active).await
            });
            None
        }
        Err(error) => Some(handle_listener_failure(error.into())),
    }
}

fn handle_listener_failure(error: anyhow::Error) -> std::time::Duration {
    tracing::warn!(detail = %format!("{error:#}"), task = HOOK_CONNECTION_TASK, "provider hook listener failed");
    crate::constants::LISTENER_RETRY_DELAY
}

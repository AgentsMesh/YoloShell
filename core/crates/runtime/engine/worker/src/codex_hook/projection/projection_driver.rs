use super::*;

pub(super) async fn run_once(
    state: Arc<WorkerState>,
    subscription: Option<CodexRouteSubscription>,
) -> Result<()> {
    let CodexRouteSubscription {
        snapshot,
        receiver: mut routes,
    } = match subscription {
        Some(subscription) => subscription,
        None => state.codex_hook_ingress.subscribe_projection_routes()?,
    };
    let (sender, mut receiver) =
        tokio::sync::mpsc::channel(super::super::constants::CODEX_PROJECTION_CHANNEL_CAPACITY);
    let mut cursor = ProjectionCursor::default();
    let mut forwarders = ProjectionForwarders::new();
    snapshot::seed(&state, &mut cursor, &mut forwarders, snapshot, &sender).await?;
    let _sender = sender.clone();
    let mut routes_closed = false;
    loop {
        if routes_closed && forwarders.tasks.is_empty() {
            while let Ok(input) = receiver.try_recv() {
                consume(&state, &mut cursor, input).await?;
            }
            return Ok(());
        }
        tokio::select! {
            route = routes.recv(), if !routes_closed => match route {
                Ok(route) => forwarders.install(route, sender.clone()),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    publication::invalidate(&state).await?;
                    bail!(super::super::constants::CODEX_PROJECTION_ROUTE_LAG);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    if !state.response_shutdown_is_committed() {
                        publication::invalidate(&state).await?;
                        bail!(super::super::constants::CODEX_PROJECTION_ROUTE_CLOSED);
                    }
                    routes_closed = true;
                }
            },
            input = receiver.recv() => {
                let Some(input) = input else {
                    bail!(super::super::constants::CODEX_PROJECTION_ROUTE_CLOSED);
                };
                consume(&state, &mut cursor, input).await?;
            }
            completed = forwarders.tasks.join_next(), if !forwarders.tasks.is_empty() => {
                completed
                    .transpose()
                    .context(super::super::constants::CODEX_PROJECTION_FORWARDER_JOIN)?;
            }
        }
    }
}

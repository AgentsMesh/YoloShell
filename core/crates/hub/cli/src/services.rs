use anyhow::Result;
use tokio::{signal::unix::Signal, sync::watch, task::JoinSet};
use yolo_hub_connector_service::ConnectorServer;
use yolo_hub_management_runtime::ManagementService;
use yolo_hub_server::HubServer;

pub(crate) async fn run(
    server: HubServer,
    connectors: ConnectorServer,
    service: ManagementService,
    public: yolo_hub_public_client_server::PublicClientServer,
    access: std::sync::Arc<yolo_hub_client_access_runtime::ClientAccessRuntime>,
    mut terminate: Signal,
) -> Result<()> {
    let (stop, stopped) = watch::channel(false);
    let mut tasks = JoinSet::new();
    let mut local_stop = stopped.clone();
    tasks.spawn(server.serve_until(async move {
        let _ = local_stop.changed().await;
    }));
    let mut connector_stop = stopped.clone();
    tasks.spawn(connectors.serve_until(async move {
        let _ = connector_stop.changed().await;
    }));
    let mut public_stop = stopped.clone();
    tasks.spawn(public.serve_until(async move {
        let _ = public_stop.changed().await;
    }));
    let access_stop = stopped.clone();
    tasks.spawn(async move { access.run(access_stop).await });
    tasks.spawn(service.run(stopped));
    let first = tokio::select! {
        _ = tokio::signal::ctrl_c() => None,
        _ = terminate.recv() => None,
        result = tasks.join_next() => result,
    };
    stop.send_replace(true);
    let mut outcome = first.map_or(Ok(()), |result| {
        result
            .map_err(anyhow::Error::from)
            .and_then(|result| result)
    });
    while let Some(result) = tasks.join_next().await {
        let result = result
            .map_err(anyhow::Error::from)
            .and_then(|result| result);
        if outcome.is_ok() {
            outcome = result;
        }
    }
    outcome
}

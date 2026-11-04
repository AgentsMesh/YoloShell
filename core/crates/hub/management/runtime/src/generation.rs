use anyhow::Result;
use std::{net::TcpListener, sync::Arc};
use tokio::{sync::watch, task::JoinHandle};
use yolo_hub_federation::{Federation, ObservationHistory};
use yolo_hub_federation_settings::Configuration;
use yolo_hub_local_backend::LocalBackend;
use yolo_hub_peer_dialer::PeerDialer;
use yolo_hub_state::HubState;

pub(crate) struct Running {
    pub node: Arc<Federation>,
    pub stop: watch::Sender<bool>,
    pub task: JoinHandle<Result<()>>,
    pub finished: watch::Receiver<bool>,
}

impl Running {
    pub async fn start(
        state: Arc<HubState>,
        backend: LocalBackend,
        configuration: Configuration,
        listener: &Option<TcpListener>,
        history: Option<ObservationHistory>,
        dialer: Arc<dyn PeerDialer>,
        origin: Option<Arc<dyn yolo_hub_dispatch::OriginDecorator<LocalBackend>>>,
    ) -> Result<Self> {
        let listener = crate::listener::activate(listener)?;
        let node = Federation::new_with_dialer(state, backend, configuration, dialer, origin).await?;
        if let Some(history) = history {
            node.restore_history(history).await?;
        }
        let (stop, stopped) = watch::channel(false);
        let (done, finished) = watch::channel(false);
        let running = node.clone();
        let task = tokio::spawn(async move {
            let result = running.run(listener, stopped).await;
            done.send_replace(true);
            result
        });
        Ok(Self {
            node,
            stop,
            task,
            finished,
        })
    }
}

pub(crate) async fn failure(running: &mut Option<Running>) {
    match running {
        Some(running) => {
            if !*running.finished.borrow() {
                let _ = running.finished.changed().await;
            }
        }
        None => std::future::pending().await,
    }
}

use crate::{Federation, constants::*, node::now_ms};
use anyhow::Result;
use std::sync::Arc;
use tokio::{
    net::TcpListener,
    sync::watch,
    task::JoinSet,
    time::{Instant, interval, timeout_at},
};

impl Federation {
    pub async fn run(
        self: Arc<Self>,
        listener: Option<TcpListener>,
        mut shutdown: watch::Receiver<bool>,
    ) -> Result<()> {
        if *shutdown.borrow() || self.is_closing() {
            self.begin_shutdown();
            self.observation_cache
                .wait_closed(Instant::now() + STOP_TIMEOUT)
                .await;
            return Ok(());
        }
        self.state.verify()?;
        let (stop, stopped) = watch::channel(false);
        let mut tasks = JoinSet::new();
        let backend = self.backend.clone();
        let host_stop = stopped.clone();
        tasks.spawn(async move {
            backend.hold_host(host_stop).await;
            Ok(())
        });
        tasks.spawn(yolo_hub_catalogue_events::run(
            self.backend.clone(),
            self.catalogue_wake.clone(),
            stopped.clone(),
        ));
        tasks.spawn(self.clone().publish(stopped.clone()));
        if let Some(listener) = listener {
            tasks.spawn(self.clone().accept_loop(listener, stopped.clone()));
        }
        for peer in self
            .config
            .peers
            .iter()
            .filter(|peer| peer.dial.is_some())
            .cloned()
        {
            tasks.spawn(self.clone().dial_loop(peer, stopped.clone()));
        }
        let result = loop {
            if *shutdown.borrow() {
                break Ok(());
            }
            tokio::select! {
                _ = shutdown.changed() => break Ok(()),
                result = tasks.join_next(), if !tasks.is_empty() => match result {
                    Some(Ok(Err(error))) => break Err(error),
                    Some(Err(error)) => break Err(error.into()),
                    _ => {},
                }
            }
        };
        self.begin_shutdown();
        stop.send_replace(true);
        let deadline = Instant::now() + STOP_TIMEOUT;
        while !tasks.is_empty() {
            if timeout_at(deadline, tasks.join_next()).await.is_err() {
                tasks.abort_all();
                break;
            }
        }
        while tasks.join_next().await.is_some() {}
        let connections = std::mem::take(&mut *self.connections.write().await);
        for link in connections.into_values() {
            let _ = self
                .book
                .lock()
                .await
                .disconnect(link.connection.peer().hub(), link.id);
            let _ = link.connection.close(Instant::now() + STOP_TIMEOUT).await;
        }
        self.observation_cache
            .wait_closed(Instant::now() + STOP_TIMEOUT)
            .await;
        result
    }
    async fn publish(self: Arc<Self>, mut stop: watch::Receiver<bool>) -> Result<()> {
        let mut tick = interval(PUBLISH_HEARTBEAT);
        let mut wake = self.catalogue_wake.subscribe();
        loop {
            tokio::select! {
                _ = stop.changed() => break,
                _ = tick.tick() => {}
                _ = wake.changed() => {}
            }
            if self.is_closing() {
                break;
            }
            self.state.verify()?;
            let mut publication = self.publication.lock().await;
            if self.is_closing() {
                break;
            }
            self.state.verify()?;
            match publication
                .refresh(&self.backend, &self.state, now_ms()?)
                .await
            {
                Ok(()) => {
                    if self.is_closing() {
                        break;
                    }
                    self.state.verify()?;
                    self.latest
                        .send_replace(publication.latest().cloned().map(Arc::new));
                }
                Err(error) => tracing::warn!(%error, message = PUBLICATION_FAILED),
            }
        }
        Ok(())
    }
}

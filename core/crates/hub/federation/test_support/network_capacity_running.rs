use super::{constants::*, fixture::LinkFixture};
use tokio::{
    sync::{oneshot, watch},
    task::JoinSet,
    time::timeout,
};

pub struct Running {
    pub finished: oneshot::Receiver<anyhow::Result<()>>,
    stop: watch::Sender<bool>,
    tasks: JoinSet<()>,
}

impl Running {
    pub fn start(fixture: &mut LinkFixture) -> Self {
        let (stop, stopped) = watch::channel(false);
        let (done, finished) = oneshot::channel();
        let mut tasks = JoinSet::new();
        let node = fixture.node.clone();
        let link = fixture.link.clone();
        let incoming = fixture.incoming.take().unwrap();
        tasks.spawn(async move {
            let _ = done.send(node.serve_link(link, incoming, stopped).await);
        });
        let mut stopped = stop.subscribe();
        let mut incoming = fixture.peer_incoming.take().unwrap();
        let responder = fixture.responder.clone();
        tasks.spawn(async move {
            let mut channels = JoinSet::new();
            loop {
                tokio::select! {
                    _ = stopped.changed() => break,
                    _ = channels.join_next(), if !channels.is_empty() => {},
                    channel = incoming.recv() => {
                        let Some(channel) = channel else { break; };
                        let responder = responder.clone();
                        channels.spawn(async move {
                            let _ = yolo_hub_peer_rpc::serve(channel, responder).await;
                        });
                    }
                }
            }
            channels.abort_all();
            while channels.join_next().await.is_some() {}
        });
        Self {
            finished,
            stop,
            tasks,
        }
    }

    pub async fn close(mut self) {
        self.stop.send_replace(true);
        timeout(TEST_TIMEOUT, async {
            while let Some(task) = self.tasks.join_next().await {
                task.unwrap();
            }
        })
        .await
        .unwrap();
    }
}

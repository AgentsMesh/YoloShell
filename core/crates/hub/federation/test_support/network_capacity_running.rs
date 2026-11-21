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
        tasks.spawn(async move {
            let _ = done.send(node.serve_link(link, stopped).await);
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

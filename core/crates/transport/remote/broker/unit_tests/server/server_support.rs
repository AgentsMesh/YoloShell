use super::super::*;
use super::support::*;
use crate::Client;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::oneshot;

const SERVER_READY_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(5);

pub struct Harness {
    _root: TempDir,
    pub paths: BrokerPaths,
    stop: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<anyhow::Result<()>>,
}

impl Harness {
    pub async fn start(factory: Arc<FakeFactory>) -> Self {
        Self::start_with_limits(
            factory,
            super::super::constants::MAX_RUNTIME_CLIENTS,
            super::super::constants::MAX_MANAGEMENT_CLIENTS,
        )
        .await
    }

    pub async fn start_with_limits(
        factory: Arc<FakeFactory>,
        runtime_limit: usize,
        management_limit: usize,
    ) -> Self {
        Self::start_with_config_and_limits(factory, test_config(), runtime_limit, management_limit)
            .await
    }

    pub async fn start_with_config(factory: Arc<FakeFactory>, config: BrokerConfig) -> Self {
        Self::start_with_config_and_limits(
            factory,
            config,
            super::super::constants::MAX_RUNTIME_CLIENTS,
            super::super::constants::MAX_MANAGEMENT_CLIENTS,
        )
        .await
    }

    pub async fn start_with_connection_factory(
        factory: Arc<dyn BrokerConnectionFactory>,
        config: BrokerConfig,
    ) -> Self {
        Self::start_with_config_and_limits(
            factory,
            config,
            super::super::constants::MAX_RUNTIME_CLIENTS,
            super::super::constants::MAX_MANAGEMENT_CLIENTS,
        )
        .await
    }

    async fn start_with_config_and_limits(
        factory: Arc<dyn BrokerConnectionFactory>,
        config: BrokerConfig,
        runtime_limit: usize,
        management_limit: usize,
    ) -> Self {
        let root = tempfile::tempdir().unwrap();
        let paths = BrokerPaths::new(root.path().join("state")).unwrap();
        let broker = RemoteBroker::with_factory(config, paths.clone(), factory);
        let (stop, receiver) = oneshot::channel();
        let task = tokio::spawn(broker.serve_until_with_limits(
            async {
                let _ = receiver.await;
            },
            runtime_limit,
            management_limit,
        ));
        for _ in 0..100 {
            if paths.management_socket().exists() {
                break;
            }
            tokio::time::sleep(SERVER_READY_POLL_INTERVAL).await;
        }
        assert!(paths.runtime_socket().exists());
        Self {
            _root: root,
            paths,
            stop,
            task,
        }
    }

    pub async fn stop(self) {
        let _ = self.stop.send(());
        self.task.await.unwrap().unwrap();
        assert!(!self.paths.runtime_socket().exists());
        assert!(!self.paths.management_socket().exists());
    }

    pub async fn wait_socket_removed(&self) {
        while self.paths.management_socket().exists() {
            tokio::task::yield_now().await;
        }
    }

    pub fn task_finished(&self) -> bool {
        self.task.is_finished()
    }

    pub async fn wait_stopped(self) {
        self.task.await.unwrap().unwrap();
        assert!(!self.paths.runtime_socket().exists());
        assert!(!self.paths.management_socket().exists());
    }

    pub async fn abort(self) {
        self.task.abort();
        assert!(self.task.await.unwrap_err().is_cancelled());
    }

    pub async fn wait_namespace_lost(&mut self) -> anyhow::Error {
        (&mut self.task).await.unwrap().err().unwrap()
    }
}

pub async fn proxy_request(socket: PathBuf, method_name: &str) -> ProxyReply {
    let response = Client::new(socket).raw(method_name, &()).await.unwrap();
    response.decode_payload().unwrap()
}

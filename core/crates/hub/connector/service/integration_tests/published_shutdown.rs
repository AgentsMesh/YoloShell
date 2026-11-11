use crate::{constants::*, fixture::*, host::Host, publisher::*};
use tokio::{
    sync::oneshot,
    time::{sleep, timeout_at},
};
use yolo_hub_agent_protocol::{Freshness, Reachability};
use yolo_hub_connector_protocol::ConnectorError;
use yolo_hub_connector_service::ConnectorServer;
use yolo_local_unix::LocalUnixStream;

#[tokio::test]
async fn shutdown_or_outer_cancellation_stales_publisher_before_replacement_can_bind() {
    for cancel in [false, true] {
        let host = Host::new();
        let server =
            ConnectorServer::bind(host.fixture.state.clone(), host.fixture.backend.clone())
                .await
                .unwrap();
        let socket = server.socket_path();
        let (stop, stopped) = oneshot::channel();
        let serving = tokio::spawn(server.serve_until(async {
            let _ = stopped.await;
        }));
        let mut channel = Channel::new(LocalUnixStream::connect(&socket).await.unwrap());
        let activation = enroll_and_publish(&host, &mut channel).await;
        assert_eq!(host.snapshot().await.fields().freshness, Freshness::Current);
        if cancel {
            serving.abort();
            assert!(serving.await.unwrap_err().is_cancelled());
            timeout_at(deadline(), async {
                while socket.exists() {
                    sleep(POLL_INTERVAL).await;
                }
            })
            .await
            .unwrap();
        } else {
            stop.send(()).unwrap();
            timeout_at(deadline(), serving)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
        }
        assert!(!socket.exists());
        assert_eq!(
            host.fixture
                .backend
                .publish_connector(activation, publication(activation))
                .await,
            Err(ConnectorError::StaleGeneration)
        );
        let stale = host.snapshot().await;
        assert_eq!(stale.cursor().target(), activation.agent);
        assert_eq!(stale.fields().freshness, Freshness::Stale);
        assert_eq!(stale.fields().reachability, Reachability::Unreachable);
        let replacement =
            ConnectorServer::bind(host.fixture.state.clone(), host.fixture.backend.clone())
                .await
                .unwrap();
        replacement.serve_until(async {}).await.unwrap();
        assert!(!socket.exists());
    }
}

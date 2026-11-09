use super::{constants::*, fixture::*, host::Host, publisher::*};
use crate::connection;
use tokio::io::AsyncWriteExt;
use tokio::time::timeout_at;
use yolo_hub_agent_protocol::{Freshness, Reachability};
use yolo_hub_connector_protocol::ConnectorError;
use yolo_local_unix::LocalUnixStream;

#[derive(Clone, Copy)]
enum Disconnect {
    Clean,
    Malformed,
    CorruptStore,
}

#[tokio::test]
async fn activated_channel_teardown_retires_generation_and_reports_cleanup_failure() {
    for disconnect in [
        Disconnect::Clean,
        Disconnect::Malformed,
        Disconnect::CorruptStore,
    ] {
        let host = Host::new();
        let (left, right) = LocalUnixStream::pair().unwrap();
        let serving = tokio::spawn(connection::serve(
            right,
            host.fixture.state.clone(),
            host.fixture.backend.clone(),
        ));
        let mut channel = Channel::new(left);
        let activation = enroll_and_publish(&host, &mut channel).await;
        let current = host.snapshot().await;
        assert_eq!(current.cursor().target(), activation.agent);
        assert_eq!(current.fields().freshness, Freshness::Current);
        match disconnect {
            Disconnect::Clean => channel.close(deadline()).await.unwrap(),
            Disconnect::Malformed => {
                channel
                    .into_stream()
                    .unwrap()
                    .write_all(&[yolo_hub_jsonrpc_transport::FRAME_DELIMITER])
                    .await
                    .unwrap();
            }
            Disconnect::CorruptStore => {
                std::fs::write(host.fixture.connector_path.join(INDEX_FILE), CORRUPT_BYTES)
                    .unwrap();
                channel.close(deadline()).await.unwrap();
            }
        }
        let result = timeout_at(deadline(), serving).await.unwrap().unwrap();
        match disconnect {
            Disconnect::Clean => result.unwrap(),
            Disconnect::Malformed => {
                assert_eq!(
                    result.unwrap_err().to_string(),
                    crate::constants::INVALID_FRAME
                );
            }
            Disconnect::CorruptStore => {
                assert!(result.is_err());
                assert!(host.fixture.backend.list_connectors().await.is_err());
                continue;
            }
        }
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
        assert!(stale.cursor().sequence() > current.cursor().sequence());
    }
}

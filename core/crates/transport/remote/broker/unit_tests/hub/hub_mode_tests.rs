use super::super::*;
use super::support::{ConnectStep, FakeConnection, FakeFactory, test_config};
use crate::remote::*;
use std::sync::Arc;

const CONNECTION_INDEX: usize = 0;
const BINARY: &[u8] = &[0xcf, 0xfa, 0xed, 0xfe, 0x0c, 0, 0, 1];
const VERSION: &str = "1.0.0";

fn hub_config() -> BrokerConfig {
    let mut config = test_config();
    config.hub_transport = true;
    config.revision = yolo_broker_hub_test_support::HASH.into();
    config
}
fn profile(config: &BrokerConfig) -> BrokerHubProfile {
    BrokerHubProfile {
        host_id: config.host_id().into(),
        profile_authority_sha256: config.profile_authority_sha256().into(),
        config_revision: config.revision().into(),
    }
}

#[tokio::test]
async fn profile_and_mode_rejection_happen_before_connect_and_hub_mode_blocks_runtime_bootstrap() {
    let mut config = hub_config();
    let expected = profile(&config);
    let factory = FakeFactory::empty(Vec::new());
    config.hub_transport = false;
    let host = BrokerConnectionPool::configured(&config, factory.clone());
    let error = host.prepare_hub_transport(&expected).await.unwrap_err();
    assert_eq!(
        error.downcast_ref::<BrokerHubError>(),
        Some(&BrokerHubError::Mode)
    );
    config.hub_transport = true;
    let pool = BrokerConnectionPool::configured(&config, factory.clone());
    let mut wrong = expected;
    wrong.host_id.push('x');
    let error = pool.prepare_hub_transport(&wrong).await.unwrap_err();
    assert_eq!(
        error.downcast_ref::<BrokerHubError>(),
        Some(&BrokerHubError::ProfileMismatch)
    );
    assert!(pool.bootstrap().await.is_err());
    assert!(pool.open_runtime(RuntimeRole::Control).await.is_err());
    assert_eq!(factory.calls(), usize::default());
    let status = pool.status().await;
    assert!(status.supports(HUB_TRANSPORT_CAPABILITY));
    assert!(!status.supports(CONNECTION_BOOTSTRAP_CAPABILITY));
}

#[tokio::test]
async fn prepare_is_only_ssh_and_every_binding_field_fences_open_without_reconnect() {
    let config = hub_config();
    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(
        vec![connection],
        vec![ConnectStep::Connection(CONNECTION_INDEX)],
    );
    let pool = BrokerConnectionPool::configured(&config, factory.clone());
    let binding = pool.prepare_hub_transport(&profile(&config)).await.unwrap();
    assert_eq!(factory.calls(), CONNECTION_INDEX + 1);
    let mut request = yolo_broker_hub_test_support::request();
    request.binding = binding.clone();
    for field in ["instance", "connection", "generation"] {
        let mut stale = request.clone();
        match field {
            "instance" => stale.binding.broker_instance_id = uuid::Uuid::new_v4(),
            "connection" => stale.binding.connection_id = uuid::Uuid::new_v4(),
            _ => stale.binding.connection_generation += 1,
        }
        let error = pool.open_hub_peer(&stale).await.err().unwrap();
        assert_eq!(
            error.downcast_ref::<BrokerHubError>(),
            Some(&BrokerHubError::StaleBinding)
        );
    }
    let observed = pool.observed_connection().await.unwrap();
    pool.invalidate(&observed).await;
    let error = pool.open_hub_peer(&request).await.err().unwrap();
    assert_eq!(
        error.downcast_ref::<BrokerHubError>(),
        Some(&BrokerHubError::StaleBinding)
    );
    assert_eq!(factory.calls(), CONNECTION_INDEX + 1);
}

struct NeverBackend;
impl super::super::factory::FactoryBackend for NeverBackend {
    fn connect(&self, _: SshConnectConfig) -> RemoteFuture<'_, Arc<dyn MultiplexedSshConnection>> {
        panic!("mode fence reached SSH connect")
    }
    fn install<'a>(
        &'a self,
        _: &'a dyn MultiplexedSshConnection,
        _: &'a RemoteRuntimeSpec,
        _: &'a RuntimeArtifact,
    ) -> RemoteFuture<'a, ()> {
        panic!("mode fence reached Host installation")
    }
}

#[tokio::test]
async fn invalid_hub_artifact_is_rejected_by_factory_and_pool_before_network_io() {
    let mut config = hub_config();
    config.artifact = Some(
        RuntimeArtifact::new(
            VERSION,
            RemoteArchitecture::Aarch64,
            BINARY,
            yolo_crypto::sha256_hex(BINARY),
        )
        .unwrap(),
    );
    let expected = profile(&config);
    let factory = Arc::new(super::super::factory::RusshConnectionFactory::with_backend(
        config.clone(),
        Arc::new(NeverBackend),
    ));
    assert!(factory.connect().await.is_err());
    let pool = BrokerConnectionPool::configured(&config, factory);
    let error = pool.prepare_hub_transport(&expected).await.unwrap_err();
    assert_eq!(
        error.downcast_ref::<BrokerHubError>(),
        Some(&BrokerHubError::Artifact)
    );
    assert!(pool.bootstrap().await.is_err());
}

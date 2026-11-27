use super::{fixture, support::RuntimeFixture};
use yolo_remote_broker::{BrokerClient, BrokerConfig, BrokerPaths};

pub use yolo_broker_hub_protocol::HUB_TRANSPORT_FIELD;
pub const EXPECTED_CONNECTIONS: usize = 1;
pub const WRONG_HOST: &str = "other-hub-profile";
pub const WRONG_REVISION: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
pub const GENERATION: u64 = 1;

pub fn fixture() -> RuntimeFixture {
    let fixture = RuntimeFixture::new();
    let mut raw: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&fixture.options.config_source).unwrap()).unwrap();
    raw[HUB_TRANSPORT_FIELD] = serde_json::Value::Bool(true);
    std::fs::write(
        &fixture.options.config_source,
        serde_json::to_vec(&raw).unwrap(),
    )
    .unwrap();
    fixture
}

pub fn profile(config: &BrokerConfig) -> yolo_remote_broker::BrokerHubProfile {
    yolo_remote_broker::BrokerHubProfile {
        host_id: config.host_id().into(),
        profile_authority_sha256: config.profile_authority_sha256().into(),
        config_revision: config.revision().into(),
    }
}

pub async fn wait_ready(client: &BrokerClient) -> yolo_remote_broker::BrokerStatus {
    tokio::time::timeout(fixture::START_TIMEOUT, async {
        loop {
            if let Ok(status) = client.status().await {
                return status;
            }
            tokio::time::sleep(yolo_remote_broker::BROKER_READY_POLL_INTERVAL).await;
        }
    })
    .await
    .unwrap()
}

pub fn client(fixture: &RuntimeFixture) -> BrokerClient {
    BrokerClient::new(
        BrokerPaths::new(&fixture.options.broker_state)
            .unwrap()
            .management_socket(),
    )
}

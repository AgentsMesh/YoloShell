use super::{constants::*, fixture::LinkFixture};
use tokio::{
    sync::OwnedSemaphorePermit,
    task::JoinHandle,
    time::{Instant, timeout},
};
use yolo_capnp_rpc::MAX_IN_FLIGHT;
use uuid::Uuid;
use yolo_hub_peer_protocol::{PEER_PROTOCOL_VERSION, PeerRequest};
use yolo_hub_peer_rpc::RpcError;

pub struct Pressure {
    _permits: OwnedSemaphorePermit,
    tasks: Vec<JoinHandle<()>>,
}

impl Pressure {
    /// Occupies every local in-flight slot with requests whose server handlers
    /// wait on the gate, so the next request observes typed local capacity.
    pub async fn rpc(fixture: &LinkFixture) -> Self {
        let gate = fixture.responder.gate.clone();
        let permits = gate
            .clone()
            .acquire_many_owned(MAX_IN_FLIGHT as u32)
            .await
            .unwrap();
        let mut tasks = Vec::new();
        for _ in 0..MAX_IN_FLIGHT {
            let client = fixture.link.client.clone();
            tasks.push(tokio::spawn(async move {
                let request = PeerRequest::Discover {
                    request_id: Uuid::new_v4(),
                    version: PEER_PROTOCOL_VERSION,
                    allowed_origins: Vec::new(),
                };
                let _ = client.request(request, Instant::now() + TEST_TIMEOUT).await;
            }));
        }
        timeout(TEST_TIMEOUT, fixture.responder.wait_admitted(MAX_IN_FLIGHT))
            .await
            .unwrap();
        let probe = PeerRequest::Discover {
            request_id: Uuid::new_v4(),
            version: PEER_PROTOCOL_VERSION,
            allowed_origins: Vec::new(),
        };
        assert!(matches!(
            fixture
                .link
                .client
                .request(probe, Instant::now() + TEST_TIMEOUT)
                .await,
            Err(RpcError::Capacity)
        ));
        Self {
            _permits: permits,
            tasks,
        }
    }
}

impl Drop for Pressure {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

use super::hub_fixture::*;
use crate::remote_broker::*;
use std::sync::{Arc, atomic::Ordering};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout_at,
};

#[tokio::test]
async fn pending_adoption_prepares_bound_ssh_but_cannot_open_until_committed() {
    let mut fixture = Harness::pending().await;
    let request = fixture.request().await;
    let status = fixture.client().status().await.unwrap();
    assert!(!status.adopted);
    assert!(
        fixture
            .client()
            .open_hub_peer(request.clone(), deadline())
            .await
            .is_err()
    );
    assert_eq!(fixture.factory.calls.load(Ordering::Relaxed), ONE_TUNNEL);
    fixture
        .adoption
        .take()
        .unwrap()
        .send(Box::new(move |instance| instance == status.instance_id))
        .ok()
        .unwrap();
    timeout_at(deadline(), async {
        while !fixture.client().status().await.unwrap().adopted {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let tunnel = fixture
        .client()
        .open_hub_peer(request, deadline())
        .await
        .unwrap();
    let (mut remote, _) = fixture.target.listener.accept().await.unwrap();
    drop(tunnel);
    assert!(
        timeout_at(deadline(), remote.read_u8())
            .await
            .unwrap()
            .is_err()
    );
    fixture.stop().await;
}

#[tokio::test]
async fn admitted_tunnel_releases_the_only_management_slot() {
    let fixture = Harness::single_management_slot().await;
    let mut request = yolo_broker_hub_test_support::request();
    request.binding = fixture
        .pool
        .prepare_hub_transport(&request.binding.profile)
        .await
        .unwrap();
    request.socket = fixture.target.path.clone();
    let mut first = fixture
        .client()
        .open_hub_peer(request.clone(), deadline())
        .await
        .unwrap();
    let (mut remote, _) = fixture.target.listener.accept().await.unwrap();
    // Wait for the promotion completion without relying on a scheduling delay.
    timeout_at(deadline(), async {
        loop {
            if fixture.client().status().await.is_ok() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    first.write_all(PAYLOAD).await.unwrap();
    let mut bytes = vec![u8::default(); PAYLOAD.len()];
    remote.read_exact(&mut bytes).await.unwrap();
    assert_eq!(bytes, PAYLOAD);
    drop(first);
    fixture.stop().await;
}

#[tokio::test]
async fn tunnel_capacity_is_checked_before_channel_creation_and_returns_correlated_failure() {
    let fixture = Harness::start().await;
    let request = fixture.request().await;
    let pool = Arc::new(BrokerConnectionPool::configured(
        &config(),
        fixture.factory.clone(),
    ));
    let (mut client, server) = crate::local_unix::LocalUnixStream::pair().unwrap();
    let request_id = uuid::Uuid::new_v4();
    let task = tokio::spawn(async move {
        crate::remote_broker::hub_peer_proxy::setup(
            server,
            &pool,
            request_id,
            request,
            Arc::new(tokio::sync::Semaphore::new(usize::default())),
        )
        .await
        .map(|value| value.is_none())
    });
    let response: BrokerResponse = crate::protocol::read_frame(&mut client).await.unwrap();
    assert_eq!(response.request_id, request_id);
    assert_eq!(
        response.error_code.as_deref(),
        Some(BrokerHubError::Capacity.code())
    );
    assert!(task.await.unwrap().unwrap());
    assert_eq!(fixture.factory.calls.load(Ordering::Relaxed), ONE_TUNNEL);
    fixture.stop().await;
}

#[tokio::test]
async fn retirement_drains_active_tunnels_and_releases_its_namespace() {
    let fixture = Harness::start().await;
    let request = fixture.request().await;
    let mut tunnel = fixture
        .client()
        .open_hub_peer(request.clone(), deadline())
        .await
        .unwrap();
    let _remote = fixture.target.listener.accept().await.unwrap();
    fixture
        .client()
        .request(BrokerRequest::Retire {
            expected_revision: request.binding.profile.config_revision,
            expected_instance_id: request.binding.broker_instance_id,
        })
        .await
        .unwrap();
    assert!(
        timeout_at(deadline(), tunnel.read_u8())
            .await
            .unwrap()
            .is_err()
    );
    drop(tunnel);
    fixture.retired().await;
}

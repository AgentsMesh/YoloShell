use super::hub_fixture::*;
use crate::remote_broker::*;
use std::sync::atomic::Ordering;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout_at,
};

#[tokio::test]
async fn one_connection_supports_independent_tunnels_and_client_recreation_without_reconnect() {
    let fixture = Harness::start().await;
    let request = fixture.request().await;
    let mut first = fixture
        .client()
        .open_hub_peer(request.clone(), deadline())
        .await
        .unwrap();
    let (mut first_remote, _) = fixture.target.accept().await.unwrap();
    let mut second = fixture
        .client()
        .open_hub_peer(request.clone(), deadline())
        .await
        .unwrap();
    let (mut second_remote, _) = fixture.target.accept().await.unwrap();
    let status = fixture.client().status().await.unwrap();
    assert_eq!(status.connection_id, Some(request.binding.connection_id));
    first.write_all(PAYLOAD).await.unwrap();
    let mut bytes = vec![u8::default(); PAYLOAD.len()];
    first_remote.read_exact(&mut bytes).await.unwrap();
    assert_eq!(bytes, PAYLOAD);
    first_remote.write_all(PAYLOAD).await.unwrap();
    first.read_exact(&mut bytes).await.unwrap();
    assert_eq!(bytes, PAYLOAD);
    first.close();
    assert!(
        timeout_at(deadline(), first_remote.read_u8())
            .await
            .unwrap()
            .is_err()
    );
    second.write_all(PAYLOAD).await.unwrap();
    second_remote.read_exact(&mut bytes).await.unwrap();
    assert_eq!(bytes, PAYLOAD);
    fixture.ssh.assert_ssh_healthy().await;
    drop(second);
    assert!(
        timeout_at(deadline(), second_remote.read_u8())
            .await
            .unwrap()
            .is_err()
    );
    let replacement = fixture
        .client()
        .prepare_hub_transport(request.binding.profile.clone(), deadline())
        .await
        .unwrap();
    assert_eq!(replacement, request.binding);
    assert_eq!(fixture.factory.calls.load(Ordering::Relaxed), ONE_TUNNEL);
    fixture.stop().await;
}

#[tokio::test]
async fn stale_identity_wrong_profile_and_rejected_endpoint_never_reconnect_or_poison_ssh() {
    let fixture = Harness::start().await;
    let request = fixture.request().await;
    let mut wrong = request.clone();
    wrong.binding.broker_instance_id = uuid::Uuid::new_v4();
    assert!(
        matches!(fixture.client().open_hub_peer(wrong, deadline()).await, Err(BrokerClientError::Remote { code, .. }) if code == BrokerHubError::StaleBinding.code())
    );
    let mut wrong = request.clone();
    wrong.binding.profile.host_id.push('x');
    assert!(
        matches!(fixture.client().open_hub_peer(wrong, deadline()).await, Err(BrokerClientError::Remote { code, .. }) if code == BrokerHubError::ProfileMismatch.code())
    );
    let mut wrong = request.clone();
    wrong.endpoint.set_port(WRONG_PORT);
    assert!(
        fixture
            .client()
            .open_hub_peer(wrong, deadline())
            .await
            .is_err()
    );
    assert_eq!(fixture.factory.calls.load(Ordering::Relaxed), ONE_TUNNEL);
    assert_eq!(
        fixture.client().status().await.unwrap().connection_id,
        Some(request.binding.connection_id)
    );
    fixture.ssh.assert_ssh_healthy().await;
    assert!(fixture.client().bootstrap().await.is_err());
    let mut legacy = crate::local_unix::LocalUnixStream::connect(fixture.paths.runtime_socket())
        .await
        .unwrap();
    assert!(
        timeout_at(deadline(), legacy.read_u8())
            .await
            .unwrap()
            .is_err()
    );
    fixture.stop().await;
}

#[tokio::test]
async fn rotation_keeps_admitted_tunnel_alive_until_its_local_owner_closes() {
    let fixture = Harness::start().await;
    let request = fixture.request().await;
    let mut tunnel = fixture
        .client()
        .open_hub_peer(request.clone(), deadline())
        .await
        .unwrap();
    let (mut remote, _) = fixture.target.accept().await.unwrap();
    fixture
        .client()
        .shutdown(request.binding.profile.config_revision)
        .await
        .unwrap();
    timeout_at(deadline(), async {
        while fixture.paths.management_socket().exists() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    tunnel.write_all(PAYLOAD).await.unwrap();
    let mut bytes = vec![u8::default(); PAYLOAD.len()];
    remote.read_exact(&mut bytes).await.unwrap();
    assert_eq!(bytes, PAYLOAD);
    drop(tunnel);
    fixture.retired().await;
}

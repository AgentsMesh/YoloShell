use crate::support::*;
use std::sync::{Arc, atomic::Ordering};
use tokio::{
    io::AsyncReadExt,
    net::TcpListener,
    time::{Instant, timeout_at},
};
use yolo_hub_client::{ClientError, EntryConnector};
use yolo_hub_jsonrpc_transport::TransportError;

#[tokio::test]
async fn cancelling_outer_handshake_closes_the_real_connected_socket() {
    let fixture = Fixture::start(Service::Echo).await;
    let listener = TcpListener::bind((HOST, u16::default())).await.unwrap();
    let mut profile = fixture.profile.clone();
    profile.port = listener.local_addr().unwrap().port();
    let entry = Arc::new(fixture.with_profile(profile));
    let connecting = entry.clone();
    let expected = fixture.profile.hub.hub_id;
    let opening = tokio::spawn(async move { connecting.open(expected, deadline()).await });
    let (mut remote, _) = timeout_at(deadline(), listener.accept())
        .await
        .unwrap()
        .unwrap();
    timeout_at(deadline(), remote.read_u8())
        .await
        .unwrap()
        .unwrap();
    opening.abort();
    assert!(opening.await.err().unwrap().is_cancelled());
    let mut remaining = Vec::new();
    timeout_at(deadline(), remote.read_to_end(&mut remaining))
        .await
        .unwrap()
        .unwrap();
    entry.close(deadline()).await.unwrap();
}

#[tokio::test]
async fn cancelled_inner_handshake_closes_outer_connection_instead_of_retaining_detached_russh_tasks()
 {
    let fixture = Fixture::start(Service::StallBeforeHubAuth).await;
    let entry = Arc::new(fixture.entry());
    let connecting = entry.clone();
    let expected = fixture.profile.hub.hub_id;
    let opening = tokio::spawn(async move { connecting.open(expected, deadline()).await });
    let _forwarded = fixture.admitted().await;
    opening.abort();
    assert!(opening.await.err().unwrap().is_cancelled());
    timeout_at(deadline(), async {
        while fixture.counts.closed.load(Ordering::SeqCst) == NO_COUNT {
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
    entry.close(deadline()).await.unwrap();
}

#[tokio::test]
async fn explicit_close_with_expired_deadline_still_synchronously_closes_live_socket() {
    let fixture = Fixture::start(Service::Echo).await;
    let entry = fixture.entry();
    let mut channel = entry
        .open(fixture.profile.hub.hub_id, deadline())
        .await
        .unwrap();
    let _ = entry.close(Instant::now()).await;
    assert!(
        timeout_at(deadline(), channel.read_u8())
            .await
            .unwrap()
            .is_err()
    );
    assert!(
        entry
            .open(fixture.profile.hub.hub_id, deadline())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn shared_deadline_bounds_stalled_outer_handshake_and_returns_deadline_error() {
    let fixture = Fixture::start(Service::Echo).await;
    let listener = TcpListener::bind((HOST, u16::default())).await.unwrap();
    let mut profile = fixture.profile.clone();
    profile.port = listener.local_addr().unwrap().port();
    let entry = fixture.with_profile(profile);
    let opening = entry.open(
        fixture.profile.hub.hub_id,
        Instant::now() + HANDSHAKE_TIMEOUT,
    );
    let (result, accepted) = tokio::join!(opening, timeout_at(deadline(), listener.accept()));
    assert!(matches!(
        result,
        Err(ClientError::Transport(TransportError::Deadline))
    ));
    let mut remote = accepted.unwrap().unwrap().0;
    let mut bytes = Vec::new();
    timeout_at(deadline(), remote.read_to_end(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    entry.close(deadline()).await.unwrap();
}

#[tokio::test]
async fn explicit_close_interrupts_an_incomplete_inner_handshake_with_the_same_socket_owner() {
    let fixture = Fixture::start(Service::StallBeforeHubAuth).await;
    let entry = Arc::new(fixture.entry());
    let connecting = entry.clone();
    let expected = fixture.profile.hub.hub_id;
    let opening = tokio::spawn(async move { connecting.open(expected, deadline()).await });
    let _forwarded = fixture.admitted().await;
    entry.close(deadline()).await.unwrap();
    assert!(
        timeout_at(deadline(), opening)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    timeout_at(deadline(), async {
        while fixture.counts.closed.load(Ordering::SeqCst) == NO_COUNT {
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
}

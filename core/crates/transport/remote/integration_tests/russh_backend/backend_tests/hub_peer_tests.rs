use super::{constants::*, hub_peer_fixture::*};
use crate::remote::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout_at,
};

#[tokio::test]
async fn hub_peer_exec_and_sftp_share_one_authenticated_connection_and_close_independently() {
    let fixture = PeerFixture::new().await;
    let command = command();
    let connection = &fixture.ssh.connection;
    let (peer, executed, canonical) = tokio::join!(
        connection.open_hub_peer(&fixture.request),
        connection.execute(&command),
        connection.canonicalize(std::path::Path::new(MEMORY_FILE)),
    );
    let (mut peer, executed, canonical) = (peer.unwrap(), executed.unwrap(), canonical.unwrap());
    for identity in [peer.identity, executed.identity, canonical.identity] {
        assert_eq!(identity.connection_id, connection.info().connection_id);
    }
    assert_ne!(peer.identity.channel_id, executed.identity.channel_id);
    assert_ne!(peer.identity.channel_id, canonical.identity.channel_id);
    assert_eq!(peer.identity.purpose, ChannelPurpose::HubPeer);
    assert_eq!(peer.peer, fixture.request.peer());
    assert_eq!(executed.value.stdout, COMMAND_STDOUT);
    let (mut socket, _) = timeout_at(deadline(), fixture.target.listener.accept())
        .await
        .unwrap()
        .unwrap();
    peer.stream.write_all(PAYLOAD).await.unwrap();
    peer.stream.flush().await.unwrap();
    let mut bytes = vec![u8::default(); PAYLOAD.len()];
    timeout_at(deadline(), socket.read_exact(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bytes, PAYLOAD);
    socket.write_all(PAYLOAD).await.unwrap();
    timeout_at(deadline(), peer.stream.read_exact(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bytes, PAYLOAD);
    let mut sibling = connection.open_hub_peer(&fixture.request).await.unwrap();
    let (mut sibling_socket, _) = timeout_at(deadline(), fixture.target.listener.accept())
        .await
        .unwrap()
        .unwrap();
    let closer = peer.stream.close_handle();
    closer.close();
    closer.close();
    assert!(peer.stream.write(PAYLOAD).await.is_err());
    assert!(peer.stream.flush().await.is_err());
    assert!(peer.stream.read_u8().await.is_err());
    peer.stream.shutdown().await.unwrap();
    assert!(
        timeout_at(deadline(), socket.read_u8())
            .await
            .unwrap()
            .is_err()
    );
    sibling.stream.write_all(PAYLOAD).await.unwrap();
    timeout_at(deadline(), sibling_socket.read_exact(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bytes, PAYLOAD);
    drop((peer, sibling));
    assert_connection_healthy(&fixture.ssh).await;
}

#[tokio::test]
async fn wrong_connection_and_server_rejection_do_not_open_or_poison_sibling_channels() {
    let fixture = PeerFixture::new().await;
    let stale = HubPeerRequest::new(
        uuid::Uuid::new_v4(),
        fixture.request.endpoint().clone(),
        fixture.request.peer(),
    )
    .unwrap();
    assert!(matches!(
        fixture.ssh.connection.open_hub_peer(&stale).await,
        Err(RemoteError::ChannelMismatch)
    ));
    let denied = HubPeerRequest::new(
        fixture.request.connection_id(),
        HubPeerEndpoint::new("/run/user/1000/yolo/denied.sock").unwrap(),
        fixture.request.peer(),
    )
    .unwrap();
    assert!(matches!(
        fixture.ssh.connection.open_hub_peer(&denied).await,
        Err(RemoteError::ChannelRejected)
    ));
    assert_connection_healthy(&fixture.ssh).await;
}

#[tokio::test]
async fn closing_hub_peer_during_incoming_burst_does_not_disconnect_ssh() {
    let started = tokio::time::Instant::now();
    let fixture = PeerFixture::new().await;
    let mut peer = observe_phase(
        &fixture.ssh,
        started,
        Phase::Open,
        fixture.ssh.connection.open_hub_peer(&fixture.request),
    )
    .await;
    let (socket, _) = observe_phase(
        &fixture.ssh,
        started,
        Phase::Accept,
        fixture.target.listener.accept(),
    )
    .await;
    observe_phase(&fixture.ssh, started, Phase::Accept, async {
        fixture.ssh.filesystem.wait_channel_open_accepted().await;
        Ok::<_, std::convert::Infallible>(())
    })
    .await;
    let channels = fixture.ssh.filesystem.hub_peer_lifecycles();
    assert_eq!(
        channels.len(),
        EXPECTED_HUB_PEER_CHANNELS,
        "{}",
        peer_diagnostic(&fixture.ssh, started, Phase::Accept)
    );
    let channel = *channels.keys().next().unwrap();
    let write = tokio::spawn(incoming_burst(socket));
    assert_eq!(
        observe_phase(
            &fixture.ssh,
            started,
            Phase::FirstByte,
            peer.stream.read_u8()
        )
        .await,
        PAYLOAD_FILL,
        "{}",
        peer_diagnostic(&fixture.ssh, started, Phase::FirstByte)
    );
    drop(peer);
    wait_peer_close(&fixture.ssh, channel, started).await;
    let closed = observe_phase(&fixture.ssh, started, Phase::ProducerClosed, async {
        write.await.map_err(std::io::Error::other)?
    })
    .await;
    assert!(
        [closed.write_error, closed.read_error]
            .into_iter()
            .flatten()
            .all(|kind| matches!(
                kind,
                std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset
            )),
        "producer={closed:?}; {}",
        peer_diagnostic(&fixture.ssh, started, Phase::ProducerClosed)
    );
    assert_connection_healthy_at(&fixture.ssh, started).await;
    assert_exact_peer_close(&fixture.ssh, channel, started);
    observe_phase(
        &fixture.ssh,
        started,
        Phase::CleanupSettled,
        fixture
            .ssh
            .connection
            .cleanup
            .settle(cleanup_settlement_timeout(NORMAL_OPERATION_TIMEOUT)),
    )
    .await;
    assert_exact_peer_close(&fixture.ssh, channel, started);
}

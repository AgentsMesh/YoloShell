use super::{constants::*, hub_peer_fixture::*};
use crate::remote::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout_at,
};
use yolo_hub_peer_transport::{PeerIo, PeerPins};

#[tokio::test]
async fn nested_hub_handshake_over_direct_tcpip_still_requires_the_exact_inner_pin() {
    for wrong_pin in [false, true] {
        let fixture = PeerFixture::new().await;
        let (local_identity, local_key) = peer_keys();
        let expected = if wrong_pin {
            peer_keys().0
        } else {
            fixture.request.peer()
        };
        let request = HubPeerRequest::new(
            fixture.request.connection_id(),
            fixture.request.endpoint(),
            expected,
        )
        .unwrap();
        let opened = fixture
            .ssh
            .connection
            .open_hub_peer(&request)
            .await
            .unwrap();
        let (socket, _) = timeout_at(deadline(), fixture.listener.accept())
            .await
            .unwrap()
            .unwrap();
        let close = opened.stream.close_handle();
        let (client, server) = tokio::join!(
            yolo_hub_peer_transport::connect_io(
                PeerIo::new(opened.stream, move || close.close()),
                local_key,
                opened.peer,
                deadline()
            ),
            yolo_hub_peer_transport::accept(
                socket,
                fixture.key.clone(),
                PeerPins::new(vec![local_identity]).unwrap(),
                deadline()
            ),
        );
        if wrong_pin {
            assert!(client.is_err());
            assert!(server.is_err());
        } else {
            let (client, _) = client.unwrap();
            let (server, mut incoming) = server.unwrap();
            assert_eq!(client.peer(), fixture.request.peer());
            assert_eq!(server.peer(), local_identity);
            let mut channel = client.open_channel(deadline()).await.unwrap();
            let mut receive = timeout_at(deadline(), incoming.recv())
                .await
                .unwrap()
                .unwrap();
            channel.write_all(PAYLOAD).await.unwrap();
            let mut bytes = vec![u8::default(); PAYLOAD.len()];
            timeout_at(deadline(), receive.read_exact(&mut bytes))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(bytes, PAYLOAD);
            client.close(deadline()).await.unwrap();
            server.close(deadline()).await.unwrap();
        }
        assert_connection_healthy(&fixture.ssh).await;
        fixture
            .ssh
            .connection
            .cleanup
            .settle(cleanup_settlement_timeout(NORMAL_OPERATION_TIMEOUT))
            .await
            .unwrap();
    }
}

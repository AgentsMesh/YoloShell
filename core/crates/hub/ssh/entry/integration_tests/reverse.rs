use crate::support::*;
use tokio::time::timeout_at;
use yolo_hub_client::EntryConnector;

#[tokio::test]
async fn pinned_outer_server_cannot_open_any_reverse_channel() {
    let fixture = Fixture::start(Service::Echo).await;
    let entry = fixture.entry();
    let channel = entry
        .open(fixture.profile.hub.hub_id, deadline())
        .await
        .unwrap();
    let server = fixture.admitted().await;
    assert!(
        timeout_at(deadline(), server.channel_open_session())
            .await
            .unwrap()
            .is_err()
    );
    assert!(
        timeout_at(deadline(), server.channel_open_agent())
            .await
            .unwrap()
            .is_err()
    );
    assert!(
        timeout_at(deadline(), server.channel_open_direct_streamlocal(SOCKET))
            .await
            .unwrap()
            .is_err()
    );
    assert!(
        timeout_at(
            deadline(),
            server.channel_open_forwarded_streamlocal(SOCKET)
        )
        .await
        .unwrap()
        .is_err()
    );
    assert!(
        timeout_at(deadline(), server.channel_open_x11(HOST, PRIVATE_PORT))
            .await
            .unwrap()
            .is_err()
    );
    assert!(
        timeout_at(
            deadline(),
            server.channel_open_direct_tcpip(HOST, PRIVATE_PORT, HOST, PRIVATE_PORT)
        )
        .await
        .unwrap()
        .is_err()
    );
    assert!(
        timeout_at(
            deadline(),
            server.channel_open_forwarded_tcpip(HOST, PRIVATE_PORT, HOST, PRIVATE_PORT)
        )
        .await
        .unwrap()
        .is_err()
    );
    drop(channel);
    entry.close(deadline()).await.unwrap();
}

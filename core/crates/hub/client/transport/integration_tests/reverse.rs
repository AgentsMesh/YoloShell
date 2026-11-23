use crate::support::*;
use tokio::time::timeout_at;

#[tokio::test]
async fn authenticated_hub_cannot_open_any_reverse_channel_to_the_public_client() {
    let (client, server) = raw::malicious_server().await;
    let handle = server.running.handle();
    assert!(
        timeout_at(deadline(), handle.channel_open_session())
            .await
            .unwrap()
            .is_err()
    );
    assert!(
        timeout_at(deadline(), handle.channel_open_agent())
            .await
            .unwrap()
            .is_err()
    );
    assert!(
        timeout_at(
            deadline(),
            handle.channel_open_direct_streamlocal(DENIED_SOCKET)
        )
        .await
        .unwrap()
        .is_err()
    );
    assert!(
        timeout_at(
            deadline(),
            handle.channel_open_forwarded_streamlocal(DENIED_SOCKET)
        )
        .await
        .unwrap()
        .is_err()
    );
    assert!(
        timeout_at(
            deadline(),
            handle.channel_open_x11(DENIED_HOST, DENIED_PORT)
        )
        .await
        .unwrap()
        .is_err()
    );
    assert!(
        timeout_at(
            deadline(),
            handle.channel_open_direct_tcpip(DENIED_HOST, DENIED_PORT, DENIED_HOST, DENIED_PORT)
        )
        .await
        .unwrap()
        .is_err()
    );
    assert!(
        timeout_at(
            deadline(),
            handle.channel_open_forwarded_tcpip(DENIED_HOST, DENIED_PORT, DENIED_HOST, DENIED_PORT)
        )
        .await
        .unwrap()
        .is_err()
    );
    assert!(!client.is_closed());
    client.close(deadline()).await.unwrap();
}

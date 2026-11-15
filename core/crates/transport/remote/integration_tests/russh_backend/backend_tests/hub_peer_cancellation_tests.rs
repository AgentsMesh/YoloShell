use super::{constants::*, hub_peer_fixture::*, memory_fs::FailurePoint};
use crate::remote::*;
use tokio::{io::AsyncReadExt, time::timeout_at};

#[tokio::test]
async fn cancelled_delayed_direct_open_closes_its_late_channel_without_losing_ssh() {
    let fixture = PeerFixture::with_timeout(SHORT_OPERATION_TIMEOUT).await;
    fixture.ssh.filesystem.fail_once(FailurePoint::ChannelOpen);
    let barrier = fixture.ssh.connection.cleanup.block_next_task_for_test();
    let connection = fixture.ssh.connection.clone();
    let request = fixture.request;
    let opening = tokio::spawn(async move { connection.open_hub_peer(&request).await.map(|_| ()) });
    fixture.ssh.filesystem.wait_channel_open_pending().await;
    opening.abort();
    assert!(opening.await.unwrap_err().is_cancelled());
    barrier.wait_entered().await;
    fixture.ssh.filesystem.release_channel_open();
    fixture.ssh.filesystem.wait_channel_open_accepted().await;
    barrier.release();
    let (mut socket, _) = timeout_at(deadline(), fixture.listener.accept())
        .await
        .unwrap()
        .unwrap();
    assert!(
        timeout_at(deadline(), socket.read_u8())
            .await
            .unwrap()
            .is_err()
    );
    assert_connection_healthy(&fixture.ssh).await;
    fixture
        .ssh
        .connection
        .cleanup
        .settle(cleanup_settlement_timeout(SHORT_OPERATION_TIMEOUT))
        .await
        .unwrap();
}

#[tokio::test]
async fn ambiguous_direct_open_timeout_retires_the_outer_connection_before_abandonment() {
    let fixture = PeerFixture::with_timeout(SHORT_OPERATION_TIMEOUT).await;
    fixture.ssh.filesystem.fail_once(FailurePoint::ChannelOpen);
    let connection = fixture.ssh.connection.clone();
    let request = fixture.request;
    let opening = tokio::spawn(async move { connection.open_hub_peer(&request).await.map(|_| ()) });
    fixture.ssh.filesystem.wait_channel_open_pending().await;
    assert!(matches!(
        opening.await.unwrap(),
        Err(RemoteError::Transport(_))
    ));
    timeout_at(deadline(), async {
        while !fixture.ssh.connection.session.is_closed() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    fixture.ssh.filesystem.release_channel_open();
    timeout_at(deadline(), fixture.ssh.filesystem.wait_connection_closed())
        .await
        .unwrap();
}

use super::hub_fixture::*;
use crate::remote_broker::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout_at,
};

#[tokio::test]
async fn client_eof_during_open_cleans_the_late_channel_without_retiring_the_ssh_generation() {
    let fixture = Harness::start().await;
    let request = fixture.request().await;
    fixture.ssh.delay_next_channel();
    let mut local = crate::local_unix::LocalUnixStream::connect(fixture.paths.management_socket())
        .await
        .unwrap();
    crate::protocol::write_frame(
        &mut local,
        &BrokerRequestEnvelope::new(BrokerRequest::OpenHubPeer(request.clone())),
    )
    .await
    .unwrap();
    timeout_at(deadline(), fixture.ssh.wait_pending_channel())
        .await
        .unwrap();
    drop(local);
    fixture.ssh.release_pending_channel();
    timeout_at(deadline(), fixture.ssh.wait_accepted_channel())
        .await
        .unwrap();
    let (mut remote, _) = timeout_at(deadline(), fixture.target.accept())
        .await
        .unwrap()
        .unwrap();
    assert!(
        timeout_at(deadline(), remote.read_u8())
            .await
            .unwrap()
            .is_err()
    );
    assert_eq!(
        fixture.client().status().await.unwrap().connection_id,
        Some(request.binding.connection_id)
    );
    fixture.ssh.assert_ssh_healthy().await;
    fixture.stop().await;
}

#[tokio::test]
async fn local_abort_after_upgrade_does_not_interrupt_sibling_command() {
    let fixture = Harness::start().await;
    let request = fixture.request().await;
    let client = fixture.client();
    let command = tokio::spawn(async move {
        client
            .run_command(BrokerCommandRequest::standard(
                crate::remote::BROKER_COMMAND_CANCELLATION_PROGRAM,
                Vec::new(),
            ))
            .await
    });
    timeout_at(deadline(), async {
        while !fixture.ssh.command_is_live() {
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
    let (mut remote, _) = fixture.target.accept().await.unwrap();
    // Inner authentication failure has the same close ownership as this immediate drop.
    drop(tunnel);
    assert!(
        timeout_at(deadline(), remote.read_u8())
            .await
            .unwrap()
            .is_err()
    );
    assert!(fixture.ssh.command_is_live());
    fixture.ssh.assert_ssh_healthy().await;
    command.abort();
    let _ = command.await;
    fixture.stop().await;
}

#[tokio::test]
async fn absolute_deadline_expiry_before_open_does_not_create_a_channel() {
    let fixture = Harness::start().await;
    let request = fixture.request().await;
    let expired = tokio::time::Instant::now();
    assert!(matches!(
        fixture.client().open_hub_peer(request, expired).await,
        Err(BrokerClientError::Deadline)
    ));
    assert!(
        tokio::time::timeout(POLL, fixture.target.accept())
            .await
            .is_err()
    );
    fixture.ssh.assert_ssh_healthy().await;
    fixture.stop().await;
}

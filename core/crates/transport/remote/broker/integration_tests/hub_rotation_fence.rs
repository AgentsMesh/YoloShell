use super::hub_fixture::*;
use crate::remote_broker::*;
use tokio::{io::AsyncReadExt, time::timeout_at};

#[tokio::test]
async fn delayed_tunnel_open_cannot_be_promoted_after_rotation_cutoff() {
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
    fixture.ssh.release_pending_channel();
    let (mut remote, _) = timeout_at(deadline(), fixture.target.listener.accept())
        .await
        .unwrap()
        .unwrap();
    assert!(
        timeout_at(deadline(), remote.read_u8())
            .await
            .unwrap()
            .is_err()
    );
    // An acknowledgment written concurrently with the cutoff grants no live proxy.
    let _: Result<BrokerResponse, _> = crate::protocol::read_frame(&mut local).await;
    assert!(
        timeout_at(deadline(), local.read_u8())
            .await
            .unwrap()
            .is_err()
    );
    fixture.retired().await;
}

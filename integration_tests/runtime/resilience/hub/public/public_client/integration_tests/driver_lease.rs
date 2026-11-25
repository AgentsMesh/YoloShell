use super::{driver::lease::Renewal, driver_lease_peer::*};
use std::time::Duration;
use yolo_runtime::protocol::{ErrorBody, ResponseEnvelope, method};

#[tokio::test]
async fn an_idle_outer_driver_keeps_its_exact_writer_across_the_original_ttl_and_stops_before_release()
 {
    let peer = Peer::new();
    let renewal = Renewal::start(peer.client.clone(), peer.target, peer.grant.clone());
    let started = tokio::time::Instant::now();
    let mut latest = peer.grant.clone();
    for _ in 0..RENEWAL_ROUNDS {
        let (stream, request) = peer.renewal().await;
        latest.lease.expires_at_ms += i64::try_from(peer.grant.renew_after_ms).unwrap();
        respond(
            stream,
            ResponseEnvelope::success(&request, &latest).unwrap(),
        )
        .await;
    }
    assert!(started.elapsed() > Duration::from_millis(peer.grant.ttl_ms));
    let final_grant = renewal.stop().await;
    assert_eq!(final_grant, latest);
    let client = peer.client.clone();
    let target = peer.target;
    let released = tokio::spawn(async move { client.release_writer(&final_grant, target).await });
    let (stream, request) = peer.request(method::RELEASE_WRITER).await;
    respond(stream, ResponseEnvelope::empty(&request)).await;
    released.await.unwrap().unwrap();
}

#[tokio::test]
async fn failed_renewal_and_changed_writer_identity_cannot_silently_complete_driver_shutdown() {
    for changed_identity in [false, true] {
        let peer = Peer::new();
        let renewal = Renewal::start(peer.client.clone(), peer.target, peer.grant.clone());
        let (stream, request) = peer.renewal().await;
        let response = if changed_identity {
            let mut changed = peer.grant.clone();
            changed.lease.generation += GENERATION_STEP;
            ResponseEnvelope::success(&request, &changed).unwrap()
        } else {
            ResponseEnvelope::failure(
                &request,
                ErrorBody::new(
                    yolo_runtime::lease::constants::code::LEASE_EXPIRED,
                    EXPIRED_DETAIL,
                    false,
                ),
            )
        };
        respond(stream, response).await;
        let completion = tokio::spawn(renewal.stop()).await.unwrap_err();
        assert!(completion.is_panic());
    }
}

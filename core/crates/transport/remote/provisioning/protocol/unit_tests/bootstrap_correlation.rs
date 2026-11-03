use yolo_hub_remote_startup_test_support::*;
use yolo_remote_provisioning_protocol::*;

#[test]
fn first_enrollment_and_pinned_reuse_accept_only_the_complete_requested_build() {
    let pinned = request();
    let ready = RemoteBootstrapReceipt::new(receipt(&pinned)).unwrap();
    ready.validate_for(&pinned).unwrap();
    let mut enrollment = pinned.fields().clone();
    enrollment.expected_peer = None;
    ready
        .validate_for(&RemoteBootstrapRequest::new(enrollment).unwrap())
        .unwrap();
    let mut v6 = ready.fields().clone();
    v6.endpoint = std::net::SocketAddr::from((std::net::Ipv6Addr::LOCALHOST, PORT));
    RemoteBootstrapReceipt::new(v6)
        .unwrap()
        .validate_for(&pinned)
        .unwrap();
    for wrong in [
        yolo_build_image::BuildImage::new(RUNTIME_DIGEST.into(), IMAGE_BYTES + 1).unwrap(),
        yolo_build_image::BuildImage::new(FOREIGN_DIGEST.into(), IMAGE_BYTES).unwrap(),
    ] {
        for build in [
            yolo_hub_build_protocol::ServiceBuild {
                host: wrong.clone(),
                ..ready.fields().build.clone()
            },
            yolo_hub_build_protocol::ServiceBuild {
                hub: wrong.clone(),
                ..ready.fields().build.clone()
            },
            yolo_hub_build_protocol::ServiceBuild {
                file_helper: wrong.clone(),
                ..ready.fields().build.clone()
            },
        ] {
            let mut changed = ready.fields().clone();
            changed.build = build;
            assert_eq!(
                RemoteBootstrapReceipt::new(changed)
                    .unwrap()
                    .validate_for(&pinned),
                Err(BootstrapError::UpgradeRequired)
            );
        }
    }
}

#[test]
fn a_receipt_cannot_be_reused_for_another_request_consumer_policy_or_remote_pin() {
    let request = request();
    let original = receipt(&request);
    let mut variants = Vec::new();
    let mut changed = original.clone();
    changed.request_id = uuid::Uuid::new_v4();
    variants.push(changed);
    let mut changed = original.clone();
    changed.consumer = peer(FOREIGN_HUB, FOREIGN_KEY);
    variants.push(changed);
    let mut changed = original.clone();
    changed.rights = [yolo_hub_routing::Right::Observe].into_iter().collect();
    variants.push(changed);
    let mut changed = original.clone();
    changed.remaining_reexports = 1;
    variants.push(changed);
    let mut changed = original.clone();
    changed.bundle_id = RUNTIME_DIGEST.into();
    variants.push(changed);
    for changed in variants {
        assert_eq!(
            RemoteBootstrapReceipt::new(changed)
                .unwrap()
                .validate_for(&request),
            Err(BootstrapError::Correlation)
        );
    }
    let mut changed = original;
    changed.peer = peer(FOREIGN_HUB, FOREIGN_KEY);
    assert_eq!(
        RemoteBootstrapReceipt::new(changed)
            .unwrap()
            .validate_for(&request),
        Err(BootstrapError::PeerMismatch)
    );
}

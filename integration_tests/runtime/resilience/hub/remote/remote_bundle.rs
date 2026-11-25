#[path = "remote_bundle_assets.rs"]
pub(super) mod assets;
#[path = "remote_bundle/constants.rs"]
mod constants;
#[path = "remote_bundle_fixture.rs"]
mod fixture;
#[path = "remote_bundle_observe.rs"]
mod inspection;
#[path = "remote_bundle_owners.rs"]
pub(super) mod owners;
#[path = "remote_bundle_process.rs"]
mod process;
#[path = "remote_bundle_survival.rs"]
mod survival;
use fixture::Fixture;
use yolo_hub_delegation::PeerIdentity;

#[tokio::test(flavor = "multi_thread")]
async fn real_bundle_publication_cold_bootstrap_and_reuse_report_the_actual_live_images() {
    let mut fixture = Fixture::new(None);
    let bundle = fixture.install(false).await;
    let request = fixture.request(bundle.clone(), None);
    let first = fixture.bootstrap(&request).await;
    let before = inspection::configuration(&fixture, &first).await;
    inspection::build(&fixture, &first).await;
    let replay = fixture.request(bundle, Some(first.fields().peer));
    let reused = fixture.bootstrap(&replay).await;
    assert_eq!(reused.fields().peer, first.fields().peer);
    assert_eq!(reused.fields().build, first.fields().build);
    assert_eq!(inspection::configuration(&fixture, &reused).await, before);
    inspection::same_processes(&fixture, &reused).await;
    fixture.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn mismatched_remote_pin_or_installed_build_cannot_replace_running_services_or_change_policy()
{
    let mut fixture = Fixture::new(None);
    let bundle = fixture.install(false).await;
    let request = fixture.request(bundle.clone(), None);
    let first = fixture.bootstrap(&request).await;
    let before = inspection::configuration(&fixture, &first).await;
    let wrong =
        PeerIdentity::new(yolo_hub_identity::HubId::new(), *first.fields().peer.key()).unwrap();
    let wrong_pin = fixture.request(bundle, Some(wrong));
    fixture.invoke(&wrong_pin).await.failed();
    inspection::same_processes(&fixture, &first).await;
    assert_eq!(inspection::configuration(&fixture, &first).await, before);
    let mismatched = fixture.install(true).await;
    let request = fixture.request(mismatched, Some(first.fields().peer));
    let rejected = fixture.invoke(&request).await;
    rejected.failed();
    let diagnostic = String::from_utf8_lossy(&rejected.diagnostic);
    assert!(
        diagnostic.contains(
            &yolo_remote_provisioning_protocol::BootstrapError::UpgradeRequired.to_string()
        ),
        "remote build mismatch diagnostic: {diagnostic}"
    );
    inspection::same_processes(&fixture, &first).await;
    inspection::build(&fixture, &first).await;
    assert_eq!(inspection::configuration(&fixture, &first).await, before);
    fixture.finish().await;
}

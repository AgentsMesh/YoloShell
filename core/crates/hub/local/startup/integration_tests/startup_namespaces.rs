use super::pin::{Fixture, deadline};
use crate::{fence::StartupFence, namespaces::StartupNamespaces};
use yolo_hub_state::HubState;
use yolo_paths::{Paths, directory::PrivateDirectory};

const REPLACED_HOST: &str = "replaced-host";

#[tokio::test]
async fn retained_namespaces_precede_endpoint_lookup_and_preserve_existing_identity() {
    let fixture = Fixture::new();
    let peer = fixture.initialize();
    assert!(!fixture.launch.host_home.exists());
    let fence = StartupFence::acquire(&fixture.launch, Some(peer), deadline())
        .await
        .unwrap();
    let namespaces = StartupNamespaces::retain(&fixture.launch, &fence).unwrap();
    assert_eq!(
        namespaces.hub_socket().unwrap(),
        HubState::socket_path_for(&fixture.launch.state).unwrap()
    );
    assert_eq!(
        namespaces.host_socket().unwrap(),
        Paths::new(&fixture.launch.host_home).host_socket().unwrap()
    );
    assert_eq!(fixture.inspect(), Some(peer));
    namespaces.verify().unwrap();
}

#[tokio::test]
async fn invalid_startup_identity_does_not_admit_a_host_namespace() {
    let fixture = Fixture::new();
    let fence = StartupFence::acquire(&fixture.launch, None, deadline())
        .await
        .unwrap();
    fixture.displace_identity();
    assert!(StartupNamespaces::retain(&fixture.launch, &fence).is_err());
    assert!(!fixture.launch.host_home.exists());
}

#[tokio::test]
async fn host_home_replacement_revokes_retained_startup_custody() {
    let fixture = Fixture::new();
    let fence = StartupFence::acquire(&fixture.launch, None, deadline())
        .await
        .unwrap();
    let namespaces = StartupNamespaces::retain(&fixture.launch, &fence).unwrap();
    std::fs::rename(
        &fixture.launch.host_home,
        fixture.root.path().join(REPLACED_HOST),
    )
    .unwrap();
    PrivateDirectory::open_or_create_strict(&fixture.launch.host_home).unwrap();
    assert!(namespaces.verify().is_err());
    assert_eq!(
        std::fs::read_dir(&fixture.launch.host_home)
            .unwrap()
            .count(),
        0
    );
    std::fs::remove_dir(&fixture.launch.host_home).unwrap();
    std::fs::rename(
        fixture.root.path().join(REPLACED_HOST),
        &fixture.launch.host_home,
    )
    .unwrap();
    namespaces.verify().unwrap();
    drop(namespaces);
}

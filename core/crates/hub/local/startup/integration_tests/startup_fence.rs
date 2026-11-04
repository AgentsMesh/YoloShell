use super::pin::*;
use crate::{
    constants::PIN_MISMATCH, ensure_running, ensure_running_with_peer, fence::StartupFence,
};
use yolo_hub_delegation::PeerIdentity;
use yolo_paths::directory::PrivateDirectory;

#[tokio::test]
async fn pinned_missing_state_never_creates_the_directory_or_identity() {
    let existing = Fixture::new();
    let peer = existing.initialize();
    let missing = Fixture::new();
    assert!(
        ensure_running_with_peer(missing.launch.clone(), Some(peer), deadline())
            .await
            .is_err()
    );
    assert!(!missing.launch.state.exists());
    assert!(!missing.launch.host_home.exists());
    PrivateDirectory::ensure(&missing.launch.state).unwrap();
    let error = ensure_running_with_peer(missing.launch.clone(), Some(peer), deadline())
        .await
        .unwrap_err();
    assert!(error.to_string().contains(PIN_MISMATCH));
    assert_eq!(std::fs::read_dir(&missing.launch.state).unwrap().count(), 0);
}

#[tokio::test]
async fn matching_hub_id_without_matching_public_key_does_not_pass_the_pin() {
    let fixture = Fixture::new();
    let peer = fixture.initialize();
    let other = Fixture::new().initialize();
    for wrong in [
        PeerIdentity::new(peer.hub(), *other.key()).unwrap(),
        PeerIdentity::new(other.hub(), *peer.key()).unwrap(),
    ] {
        let before = std::fs::read(fixture.identity_path()).unwrap();
        let error = ensure_running_with_peer(fixture.launch.clone(), Some(wrong), deadline())
            .await
            .unwrap_err();
        assert!(error.to_string().contains(PIN_MISMATCH));
        assert_eq!(std::fs::read(fixture.identity_path()).unwrap(), before);
    }
    assert!(!fixture.launch.host_home.exists());
}

#[tokio::test]
async fn fence_preserves_local_identity_creation_and_detects_changes_before_later_commits() {
    let fixture = Fixture::new();
    let fence = StartupFence::acquire(&fixture.launch, None, deadline())
        .await
        .unwrap();
    assert_eq!(fixture.inspect(), Some(fence.peer));
    fence.verify().unwrap();
    let peer = fence.peer;
    drop(fence);
    let fence = StartupFence::acquire(&fixture.launch, Some(peer), deadline())
        .await
        .unwrap();
    fence.verify().unwrap();
    fixture.displace_identity();
    assert!(
        fence
            .verify()
            .unwrap_err()
            .to_string()
            .contains(PIN_MISMATCH)
    );
    assert_eq!(fixture.inspect(), None);
}

#[tokio::test]
async fn expired_local_and_pinned_startup_leave_missing_state_untouched() {
    let fixture = Fixture::new();
    assert!(
        ensure_running(fixture.launch.clone(), tokio::time::Instant::now())
            .await
            .is_err()
    );
    let peer = Fixture::new().initialize();
    assert!(
        ensure_running_with_peer(
            fixture.launch.clone(),
            Some(peer),
            tokio::time::Instant::now()
        )
        .await
        .is_err()
    );
    assert!(!fixture.launch.state.exists());
    assert!(!fixture.launch.host_home.exists());
}

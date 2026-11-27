use super::pin::*;
use yolo_paths::directory::PrivateDirectory;

#[tokio::test]
async fn retained_parent_cannot_follow_a_replaced_path_or_another_launch_directory() {
    let fixture = Fixture::new();
    let peer = fixture.initialize();
    let parent = PrivateDirectory::open(&fixture.launch.state).unwrap();
    std::fs::rename(
        &fixture.launch.state,
        fixture.root.path().join(SAVED_DIRECTORY),
    )
    .unwrap();
    PrivateDirectory::ensure(&fixture.launch.state).unwrap();
    assert!(
        crate::ensure_running_with_peer_in(fixture.launch.clone(), Some(peer), parent, deadline())
            .await
            .is_err()
    );
    assert_eq!(fixture.inspect(), None);
    assert!(!fixture.launch.host_home.exists());
    let other = Fixture::new();
    let peer = other.initialize();
    let parent = PrivateDirectory::open(&other.launch.state).unwrap();
    assert!(
        crate::ensure_running_with_peer_in(fixture.launch.clone(), Some(peer), parent, deadline())
            .await
            .is_err()
    );
    assert_eq!(fixture.inspect(), None);
    let parent = PrivateDirectory::open(&other.launch.state).unwrap();
    let mut relative = other.launch.clone();
    relative.state = STATE_NAME.into();
    assert!(
        crate::ensure_running_with_peer_in(relative, Some(peer), parent, deadline())
            .await
            .is_err()
    );
}

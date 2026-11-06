use super::fixture::*;
use tokio::time::{Instant, timeout_at};
use yolo_tui_tab::TabStore;
use yolo_tui_client_control::{ClientControl, detach};

#[tokio::test]
async fn long_client_path_detaches_only_after_the_owner_releases_its_store() {
    let fixture = Fixture::new(true);
    let store = fixture.store();
    let mut owner = ClientControl::bind(&store).await.unwrap();
    fixture.verify_descriptor();
    assert!(fixture.socket().as_os_str().len() < fixture.path.as_os_str().len());
    let deadline = Instant::now() + TEST_DEADLINE;
    let path = fixture.path.clone();
    let client = tokio::spawn(async move { detach(&path).await });
    timeout_at(deadline, owner.detached())
        .await
        .unwrap()
        .unwrap();
    assert!(!client.is_finished());
    assert!(TabStore::open(&fixture.path, fixture.hub).is_err());
    let socket = fixture.socket();
    drop(store);
    timeout_at(deadline, owner.finish()).await.unwrap().unwrap();
    timeout_at(deadline, client)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let reopened = fixture.store();
    assert!(!fixture.descriptor_path().exists());
    assert!(!socket.exists());
    drop(reopened);
}

#[tokio::test]
async fn second_bind_and_owner_are_rejected_without_changing_the_first_endpoint() {
    let fixture = Fixture::new(false);
    let store = fixture.store();
    let owner = ClientControl::bind(&store).await.unwrap();
    let original = fixture.descriptor();
    assert!(ClientControl::bind(&store).await.is_err());
    assert!(TabStore::open(&fixture.path, fixture.hub).is_err());
    assert_eq!(fixture.descriptor(), original);
    drop(store);
    owner.finish().await.unwrap();
    let store = fixture.store();
    ClientControl::bind(&store)
        .await
        .unwrap()
        .finish()
        .await
        .unwrap();
}

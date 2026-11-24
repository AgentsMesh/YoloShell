use super::fixture::*;
use yolo_tui_client_control::ClientControl;

#[tokio::test]
async fn dropping_owner_preserves_replaced_descriptor_and_socket() {
    let fixture = Fixture::new(false);
    let store = fixture.store();
    let owner = ClientControl::bind(&store).await.unwrap();
    let descriptor = fixture.descriptor_path();
    let socket = fixture.socket();
    let saved = fixture.root.path().join(SAVED_ENTRY);
    std::fs::rename(&descriptor, &saved).unwrap();
    std::fs::write(&descriptor, REPLACEMENT_CONTENT).unwrap();
    std::fs::remove_file(&socket).unwrap();
    std::fs::write(&socket, REPLACEMENT_CONTENT).unwrap();
    drop(owner);
    assert_eq!(std::fs::read(&descriptor).unwrap(), REPLACEMENT_CONTENT);
    assert_eq!(std::fs::read(&socket).unwrap(), REPLACEMENT_CONTENT);
    // The replacement is fixture-owned and outside TempDir; remove this exact file only.
    std::fs::remove_file(&socket).unwrap();
}

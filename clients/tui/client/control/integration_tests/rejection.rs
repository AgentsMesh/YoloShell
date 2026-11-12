use super::{exchange::*, fixture::*};
use tokio::time::{Instant, timeout_at};
use yolo_tui_client_control::{ClientControl, detach, status};
use yolo_tui_client_control_protocol::{ControlRequest, MAX_CONTROL_FRAME_BYTES};

#[tokio::test]
async fn wrong_token_version_and_malformed_requests_never_detach_the_owner() {
    let fixture = Fixture::new(false);
    let store = fixture.store();
    let mut owner = ClientControl::bind(&store).await.unwrap();
    let descriptor = fixture.descriptor();
    let mut wrong_version = ControlRequest::new(descriptor.token);
    wrong_version.version = u16::MAX;
    let mut unknown = serde_json::to_value(ControlRequest::new(descriptor.token)).unwrap();
    unknown["unexpected"] = true.into();
    for bytes in [
        frame(&ControlRequest::new(uuid::Uuid::new_v4())),
        frame(&ControlRequest::new(uuid::Uuid::nil())),
        frame(&wrong_version),
        frame(&unknown),
        u32::try_from(MAX_CONTROL_FRAME_BYTES + OVERFLOW_BYTE)
            .unwrap()
            .to_be_bytes()
            .to_vec(),
        u32::MIN.to_be_bytes().to_vec(),
        vec![1],
    ] {
        assert!(!raw(&fixture.socket(), &bytes).await.accepted);
        assert!(status(&fixture.path).await.unwrap());
    }
    let path = fixture.path.clone();
    let client = tokio::spawn(async move { detach(&path).await });
    let deadline = Instant::now() + TEST_DEADLINE;
    timeout_at(deadline, owner.detached())
        .await
        .unwrap()
        .unwrap();
    drop(store);
    timeout_at(deadline, owner.finish()).await.unwrap().unwrap();
    timeout_at(deadline, client)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(!status(&fixture.path).await.unwrap());
}

#[tokio::test]
async fn private_descriptor_validation_rejects_symlinks_public_modes_and_oversize() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let fixture = Fixture::new(false);
    let store = fixture.store();
    let owner = ClientControl::bind(&store).await.unwrap();
    let original = fixture.descriptor();
    let path = fixture.descriptor_path();
    let saved = fixture.root.path().join(SAVED_ENTRY);
    std::fs::rename(&path, &saved).unwrap();
    symlink(&saved, &path).unwrap();
    assert!(status(&fixture.path).await.is_err());
    std::fs::remove_file(&path).unwrap();
    std::fs::write(&path, vec![b'x'; MAX_CONTROL_FRAME_BYTES + OVERFLOW_BYTE]).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(PRIVATE_MODE)).unwrap();
    assert!(status(&fixture.path).await.is_err());
    std::fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(PUBLIC_MODE)).unwrap();
    assert!(status(&fixture.path).await.is_err());
    drop(owner);
    assert!(path.exists());
}

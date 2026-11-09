use super::{
    listener_constants::*, listener_initialization_fixture::InitialStartup, listener_restart::*,
};
use crate::*;
use yolo_hub_listener_binding::{LISTENER_BINDING_FILE, ListenerBindings};

#[tokio::test]
async fn startup_resumes_when_initial_configuration_committed_before_binding_creation() {
    let startup = InitialStartup::new();
    assert!(!startup.store.listener_binding_initialized().unwrap());
    assert!(
        !startup
            .state
            .directory()
            .unwrap()
            .path()
            .join(LISTENER_BINDING_FILE)
            .exists()
    );
    let (directory, backend, original) = startup.release();
    let mut resumed = reopen(directory, backend).await.unwrap();
    let applied = resumed.service.snapshot().unwrap();
    assert_eq!(applied.state, original);
    assert!(applied.applied);
    assert!(
        resumed
            .service
            .store
            .listener_binding_initialized()
            .unwrap()
    );
    resumed.service.retire().await.unwrap();
}

#[tokio::test]
async fn startup_reuses_published_binding_when_initialization_commit_was_interrupted() {
    let startup = InitialStartup::new();
    let configuration = startup.store.current().unwrap();
    let mut bindings = ListenerBindings::open(
        startup.state.directory().unwrap(),
        startup.state.peer_identity(),
        &configuration,
        !startup.store.listener_binding_initialized().unwrap(),
    )
    .unwrap();
    let listener = crate::listener::restore(&mut bindings, &configuration).unwrap();
    let allocated = crate::listener::address(&listener).unwrap();
    let path = startup
        .state
        .directory()
        .unwrap()
        .path()
        .join(LISTENER_BINDING_FILE);
    let binding_bytes = std::fs::read(&path).unwrap();
    assert!(!startup.store.listener_binding_initialized().unwrap());
    drop((bindings, listener));
    let (directory, backend, original) = startup.release();
    let mut resumed = reopen(directory, backend).await.unwrap();
    assert_eq!(resumed.service.snapshot().unwrap().state, original);
    assert_eq!(resumed.service.listen_address().unwrap(), allocated);
    assert_eq!(std::fs::read(path).unwrap(), binding_bytes);
    assert!(
        resumed
            .service
            .store
            .listener_binding_initialized()
            .unwrap()
    );
    resumed.service.retire().await.unwrap();
}

#[tokio::test]
async fn initialized_dynamic_listener_cannot_recreate_deleted_binding_on_restart() {
    let startup = InitialStartup::new();
    let (directory, backend, _) = startup.release();
    let fixture = reopen(directory, backend).await.unwrap();
    assert!(
        fixture
            .service
            .store
            .listener_binding_initialized()
            .unwrap()
    );
    let path = fixture
        .state
        .directory()
        .unwrap()
        .path()
        .join(LISTENER_BINDING_FILE);
    let (directory, backend) = stopped(fixture).await;
    std::fs::rename(&path, directory.path().join(SAVED_BINDING)).unwrap();
    let state = Arc::new(
        HubState::open(
            &directory.path().join(HUB_DIRECTORY),
            &directory.path().join(HOST_DIRECTORY),
        )
        .unwrap(),
    );
    assert!(ManagedHub::open(state, backend, None).await.is_err());
    assert!(!path.exists());
}

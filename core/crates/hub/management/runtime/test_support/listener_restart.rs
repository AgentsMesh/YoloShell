use super::{fixture::Fixture, listener_constants::*};
use crate::*;

pub async fn stopped(fixture: Fixture) -> (tempfile::TempDir, LocalBackend) {
    let Fixture {
        directory,
        state,
        hub,
        mut service,
    } = fixture;
    let backend = service.backend.clone();
    service.retire().await.unwrap();
    drop((service, hub, state));
    (directory, backend)
}

pub async fn reopen(directory: tempfile::TempDir, backend: LocalBackend) -> Result<Fixture> {
    let state = Arc::new(HubState::open(
        &directory.path().join(HUB_DIRECTORY),
        &directory.path().join(HOST_DIRECTORY),
    )?);
    let (hub, service) = ManagedHub::open(state.clone(), backend, None).await?;
    Ok(Fixture {
        directory,
        state,
        hub,
        service,
    })
}

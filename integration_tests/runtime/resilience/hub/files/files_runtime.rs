use crate::Harness;
use std::path::PathBuf;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_local_backend::LocalBackend;
use yolo_hub_state::HubState;

const FILE_HELPER_ENV: &str = "YOLO_TEST_FILE_HELPER_BINARY";

pub fn helper() -> PathBuf {
    crate::bazel_runfile(&std::env::var(FILE_HELPER_ENV).unwrap()).unwrap()
}
pub fn backend(harness: &Harness, state: &HubState) -> LocalBackend {
    let backend = LocalBackend::new(
        state.hub_id(),
        harness.home.path(),
        &harness.paths().host_socket().unwrap(),
    )
    .unwrap();
    crate::hub::fixture_agents::with_fixture_agents(backend, state)
        .with_file_helper(helper())
        .unwrap()
}
pub async fn prepared(
    harness: &Harness,
    state: &HubState,
    configuration: &Configuration,
) -> LocalBackend {
    backend(harness, state)
        .prepare_files(state.directory().unwrap().path(), &configuration.file_roots)
        .await
        .unwrap()
}

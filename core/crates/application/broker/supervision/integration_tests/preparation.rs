use crate::{ChildLaunch, CredentialSource, serve};
use yolo_remote_broker::BrokerPaths;
#[path = "../test_support/preparation.rs"]
mod fixture;
use fixture::RuntimeFixture;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use yolo_remote_broker::stage_broker_config;

const IMMUTABLE_FILE_MODE: u32 = 0o400;
const REVISION_MISMATCH_MESSAGE: &str = "does not match expected revision";
const ANCHOR_MISMATCH_MESSAGE: &str = "differs from expected anchor";
const DISPLACED_STATE_DIRECTORY: &str = "expected-anchor-displaced-state";
const SENTINEL_FILE: &str = "replacement-sentinel";
const SENTINEL_BYTES: &[u8] = b"preserve-replacement";

fn set_mode(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

#[tokio::test]
async fn serve_rejects_a_generation_replaced_after_staging_before_binding() {
    let fixture = RuntimeFixture::new();
    let paths = BrokerPaths::new(&fixture.state_dir).unwrap();
    let generation_a = stage_broker_config(&fixture.config, &paths).unwrap();
    let mut source: serde_json::Value =
        serde_json::from_slice(&fs::read(&fixture.config).unwrap()).unwrap();
    source[fixture::ENDPOINT_FIELD][fixture::PORT_FIELD] =
        serde_json::json!(fixture::REPLACEMENT_PORT);
    fs::write(&fixture.config, serde_json::to_vec(&source).unwrap()).unwrap();
    let generation_b = stage_broker_config(&fixture.config, &paths).unwrap();
    assert_ne!(generation_a.revision(), generation_b.revision());

    set_mode(generation_a.config_path(), fixture::PRIVATE_FILE_MODE);
    fs::write(
        generation_a.config_path(),
        fs::read(generation_b.config_path()).unwrap(),
    )
    .unwrap();
    set_mode(generation_a.config_path(), IMMUTABLE_FILE_MODE);
    let launch_deadline = tokio::time::Instant::now() + fixture::START_TIMEOUT;
    let guard = yolo_remote_broker::BrokerReconcileGuard::acquire(&paths, launch_deadline)
        .await
        .unwrap();
    let authority = yolo_remote_broker::authorize_broker_launch(&paths, &guard, launch_deadline)
        .await
        .unwrap();
    drop(guard);
    let launch = ChildLaunch::new(
        generation_a.config_path().to_owned(),
        fixture.state_dir.clone(),
        generation_a.revision().to_owned(),
        paths.state_anchor(),
        authority,
        yolo_remote_broker::BrokerLaunchDeadline::capture(launch_deadline).unwrap(),
    )
    .unwrap();
    let error = serve(launch, CredentialSource::None).await.unwrap_err();

    assert!(error.to_string().contains(REVISION_MISMATCH_MESSAGE));
    assert!(!paths.runtime_socket().exists());
    assert!(!paths.management_socket().exists());
}

#[tokio::test]
async fn serve_rejects_replacement_state_before_loading_or_binding() {
    let fixture = RuntimeFixture::new();
    let paths = BrokerPaths::new(&fixture.state_dir).unwrap();
    let generation = stage_broker_config(&fixture.config, &paths).unwrap();
    let anchor = paths.state_anchor();
    let displaced = paths
        .state_dir()
        .parent()
        .unwrap()
        .join(DISPLACED_STATE_DIRECTORY);
    fs::rename(paths.state_dir(), displaced).unwrap();
    let replacement = BrokerPaths::new(paths.state_dir()).unwrap();
    let sentinel = replacement.state_dir().join(SENTINEL_FILE);
    fs::write(&sentinel, SENTINEL_BYTES).unwrap();

    let launch = ChildLaunch::new(
        generation.config_path().to_owned(),
        replacement.state_dir().to_owned(),
        generation.revision().to_owned(),
        anchor,
        uuid::Uuid::new_v4().to_string().parse().unwrap(),
        yolo_remote_broker::BrokerLaunchDeadline::capture(
            tokio::time::Instant::now() + fixture::START_TIMEOUT,
        )
        .unwrap(),
    )
    .unwrap();
    let error = serve(launch, CredentialSource::None).await.unwrap_err();

    assert!(error.to_string().contains(ANCHOR_MISMATCH_MESSAGE));
    assert_eq!(fs::read(sentinel).unwrap(), SENTINEL_BYTES);
    assert!(!replacement.runtime_socket().exists());
    assert!(!replacement.management_socket().exists());
}

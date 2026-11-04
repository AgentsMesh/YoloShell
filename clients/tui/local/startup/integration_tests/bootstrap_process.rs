use super::{bootstrap_contract::*, bootstrap_process_support::*};
use std::fs;
use tokio::time::Instant;

#[tokio::test]
async fn malformed_and_unsuccessful_helpers_never_publish_a_connection() {
    for mode in [MALFORMED, FAILED] {
        let (installation, profile) = fixture(mode);
        assert!(
            crate::prepare(&profile, &installation.executable)
                .await
                .is_err()
        );
        assert!(!profile.host_home.exists());
    }
}

#[tokio::test]
async fn oversized_output_retires_the_exact_owned_helper_before_profile_cleanup() {
    let (installation, profile) = fixture(OVERFLOW);
    let deadline = Instant::now() + yolo_hub_local_startup::STARTUP_TIMEOUT;
    let mut bootstrap = Box::pin(crate::prepare(&profile, &installation.executable));
    let identity = tokio::select! {
        result = &mut bootstrap => panic!("fixture unexpectedly completed: {result:?}"),
        identity = wait_ready(&profile.state, deadline) => identity,
    };
    fs::write(profile.state.join(RELEASE_FILE), []).unwrap();
    let error = bootstrap.await.unwrap_err();
    assert_eq!(error.to_string(), crate::constants::OUTPUT_TOO_LARGE);
    settle_aborted_helper(identity, deadline).await;
    assert!(!profile.host_home.exists());
}

#[tokio::test]
async fn cancelling_a_pending_bootstrap_retires_only_the_owned_helper() {
    let (installation, profile) = fixture(HOLD);
    let deadline = Instant::now() + yolo_hub_local_startup::STARTUP_TIMEOUT;
    let mut bootstrap = Box::pin(crate::prepare(&profile, &installation.executable));
    let identity = tokio::select! {
        result = &mut bootstrap => panic!("fixture unexpectedly completed: {result:?}"),
        identity = wait_ready(&profile.state, deadline) => identity,
    };
    drop(bootstrap);
    settle_aborted_helper(identity, deadline).await;
    assert!(!profile.host_home.exists());
}

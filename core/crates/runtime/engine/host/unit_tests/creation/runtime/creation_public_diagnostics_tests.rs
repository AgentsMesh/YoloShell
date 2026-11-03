use crate::host::tests::*;
const PRIVATE_WORKER_PATH_COMPONENT: &str = "private-worker-token-sk_live_secret";
const EXPECTED_PRIVATE_WORKER_START_FAILURE: &str = "private worker start unexpectedly succeeded";

#[tokio::test]
async fn creation_worker_start_error_does_not_disclose_private_diagnostics() {
    let mut fixture = CreationCheckpointFixture::new(false).await;
    let private_path = fixture
        ._directory
        .path()
        .join(PRIVATE_WORKER_PATH_COMPONENT);
    fixture.host.runtime_executable = private_path.clone();
    let mut manifest = fixture.manifest();

    let error = match fixture
        .host
        .launch_bound_session(
            &mut manifest,
            &fixture.binding,
            &fixture.planned,
            HostOperationDeadline::after(creation_launch_test_timeout(HOST_BLOCKING_TEST_TIMEOUT)),
        )
        .await
    {
        Err(error) => error,
        Ok(_) => panic!("{EXPECTED_PRIVATE_WORKER_START_FAILURE}"),
    };

    assert_eq!(error.code, crate::protocol::error_code::WORKER_START_FAILED);
    assert_eq!(error.message, WORKER_START_PUBLIC_DETAIL);
    assert!(!error.message.contains(PRIVATE_WORKER_PATH_COMPONENT));
    assert!(
        !error
            .message
            .contains(private_path.to_string_lossy().as_ref())
    );
}

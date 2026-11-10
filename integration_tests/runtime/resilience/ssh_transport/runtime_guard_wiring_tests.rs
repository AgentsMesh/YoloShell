use super::*;
use yolo_runtime::protocol::error_code;
const EXPECTED_BOUND_RUNTIME_COUNT: usize = 1;
const EXPECTED_PENDING_RUNTIME_COUNT: usize = 0;
const CREATION_RETRY_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(25);
const CREATION_RETRY_TIMEOUT: std::time::Duration =
    yolo_remote_broker::EXISTING_SOCKET_CLIENT_OPERATION_TIMEOUT;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn existing_detached_host_inventory_is_bound_during_startup() {
    let sshd = SshdFixture::start().await;
    let runtime =
        RemoteRuntimeSpec::new(runtime_binary().canonicalize().unwrap(), sshd.remote_home())
            .unwrap();
    let first_factory = factory(&sshd);
    let (first_connection, first_opened, mut first_guard) =
        open_initial(&sshd, &first_factory, &runtime).await;
    let mut first_control = first_opened.stream;
    let created = create_running_session(&sshd, &mut first_control).await;
    first_guard.disarm_for_test();
    drop((first_control, first_connection, first_guard));

    let second_factory = factory(&sshd);
    let (second_connection, second_opened, mut second_guard) =
        open_initial(&sshd, &second_factory, &runtime).await;
    let (bound, pending, _) = second_guard.cleanup_authority_counts_for_test();
    assert_eq!(bound, EXPECTED_BOUND_RUNTIME_COUNT);
    assert_eq!(pending, EXPECTED_PENDING_RUNTIME_COUNT);
    assert!(second_guard.cleanup_tree_contains_view_for_test(&created));
    let mut second_control = second_opened.stream;
    let restored = wait_for_running(&mut second_control, created.manifest.terminal_id).await;
    assert_runtime_identity(&created, &restored);

    drop((second_control, second_connection));
    second_guard.shutdown().await.unwrap();
}

fn factory(sshd: &SshdFixture) -> Arc<LiveSshFactory> {
    let endpoint = RemoteEndpoint::new(sshd.user(), LOOPBACK_HOST, sshd.address().port()).unwrap();
    LiveSshFactory::new(
        endpoint,
        sshd.known_hosts(sshd.address().port()),
        sshd.identity(),
    )
}

async fn create_running_session(
    sshd: &SshdFixture,
    control: &mut yolo_runtime_rpc::RuntimeConnection,
) -> TerminalView {
    let creation_token = Uuid::new_v4();
    let creation = CreateTerminal {
        creation_token,
        name: Some(TEST_SESSION_NAME.into()),
        cwd: sshd.remote_home().to_path_buf(),
        cols: Some(yolo_runtime::model::DEFAULT_COLS),
        rows: Some(yolo_runtime::model::DEFAULT_ROWS),
    };
    let created = create_idempotently(control, &creation).await;
    let _: () = request(
        control,
        method::FINALIZE_CREATION,
        &FinalizeCreationRequest {
            creation_token,
            terminal_id: created.terminal_id,
            creation_capability: created.creation_capability,
        },
    )
    .await;
    wait_for_running(control, created.terminal_id).await
}

async fn create_idempotently(
    control: &mut yolo_runtime_rpc::RuntimeConnection,
    request: &CreateTerminal,
) -> CreateTerminalResult {
    let deadline = tokio::time::Instant::now() + CREATION_RETRY_TIMEOUT;
    loop {
        match request_outcome(control, method::CREATE_SESSION, request)
            .await
            .unwrap()
        {
            CheckedResponse::Success(created) => return created,
            CheckedResponse::Failure(error)
                if error.retryable
                    && error.code == error_code::DELIVERY_UNKNOWN
                    && tokio::time::Instant::now() < deadline =>
            {
                tokio::time::sleep(CREATION_RETRY_POLL_INTERVAL).await;
            }
            CheckedResponse::Failure(error) => panic!("remote creation failed: {error:?}"),
        }
    }
}

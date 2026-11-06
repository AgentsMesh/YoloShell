use super::*;

const DISCARD_FAILURE_DETAIL: &str = "discard failed with retained runtime evidence";

fn creation_request(token: Uuid, name: &str, cwd: PathBuf) -> CreateTerminal {
    CreateTerminal {
        creation_token: token,
        name: Some(name.into()),
        cwd,
        cols: Some(90),
        rows: Some(25),
        initial_input: None,
        context: None,
    }
}

fn assert_remote_code(error: ClientError, expected: &str) {
    assert!(
        matches!(error, ClientError::Remote { ref code, .. } if code == expected),
        "unexpected client error: {error:?}"
    );
}

async fn discard_with_diagnostics(
    harness: &Harness,
    terminal_id: Uuid,
) -> yolo_runtime::TerminalView {
    match harness.client().discard(terminal_id, true).await {
        Ok(view) => view,
        Err(error) => {
            let paths = harness.paths();
            let runtime = read_json::<RuntimeRecord>(&paths.runtime_record(terminal_id));
            let exit = std::fs::read_to_string(paths.exit_record(terminal_id));
            let processes = runtime.as_ref().ok().map(|record| {
                let child = process::ProcessIdentity {
                    pid: record.child_pid,
                    start_time: record.child_start_time,
                };
                (
                    process::try_process_identity_matches(
                        record.worker_pid,
                        record.worker_start_time,
                    ),
                    process::try_process_identity_matches(
                        record.child_pid,
                        record.child_start_time,
                    ),
                    child.is_stably_stopped(),
                    process::observe_process_tree(child),
                )
            });
            let view = harness.client().get_session(terminal_id).await;
            panic!(
                "{DISCARD_FAILURE_DETAIL}: error={error:?}; runtime={runtime:?}; \
                 processes={processes:?}; exit={exit:?}; view={view:?}; host_stderr={}",
                harness.host_stderr_tail()
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn creation_token_replays_concurrently_across_cwd_loss_and_restart() {
    let mut harness = Harness::start().await;
    let cwd = harness.home.path().join("replay-cwd");
    std::fs::create_dir(&cwd).unwrap();
    let token = Uuid::new_v4();
    let request = creation_request(token, "replayed", cwd.clone());
    let first_client = harness.client();
    let second_client = harness.client();
    let (first, second) = tokio::join!(
        first_client.create_session(&request),
        second_client.create_session(&request)
    );
    let first = first.unwrap();
    let second = second.unwrap();
    assert_eq!(first, second);
    assert_ne!(first.creation_capability, token);
    assert_eq!(
        harness.client().list_sessions(false).await.unwrap().len(),
        1
    );
    let trusted = harness
        .client()
        .get_session(first.terminal_id)
        .await
        .unwrap();
    harness.track_view(&trusted);

    let ledger =
        std::fs::read_to_string(harness.home.path().join("creation-capabilities.json")).unwrap();
    assert!(!ledger.contains(&token.to_string()));
    std::fs::remove_dir(&cwd).unwrap();
    assert_eq!(
        harness.client().create_session(&request).await.unwrap(),
        first
    );

    harness.restart_host_hard().await;
    assert_eq!(
        harness.client().create_session(&request).await.unwrap(),
        first
    );
    let mut changed = request.clone();
    changed.name = Some("changed".into());
    assert_remote_code(
        harness.client().create_session(&changed).await.unwrap_err(),
        error_code::CONFLICT,
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn finalize_is_triple_bound_idempotent_and_retires_cleanup_authority() {
    let harness = Harness::start().await;
    let view = harness.create_unfinalized_cat("finalized").await;
    let terminal_id = view.manifest.terminal_id;
    let (token, capability) = harness.creation_credentials(terminal_id);
    assert_remote_code(
        harness
            .client()
            .finalize_creation(token, terminal_id, Uuid::new_v4())
            .await
            .unwrap_err(),
        error_code::UNAUTHORIZED,
    );
    harness
        .client()
        .finalize_creation(token, terminal_id, capability)
        .await
        .unwrap();
    harness
        .client()
        .finalize_creation(token, terminal_id, capability)
        .await
        .unwrap();
    assert_remote_code(
        harness
            .client()
            .cleanup_terminate(terminal_id, capability, true)
            .await
            .unwrap_err(),
        error_code::CONFLICT,
    );
    let replay = creation_request(token, "finalized", harness.home.path().to_path_buf());
    assert_remote_code(
        harness.client().create_session(&replay).await.unwrap_err(),
        error_code::CONFLICT,
    );
    assert!(
        discard_with_diagnostics(&harness, terminal_id)
            .await
            .manifest
            .archived
    );
    assert!(
        discard_with_diagnostics(&harness, terminal_id)
            .await
            .manifest
            .archived
    );
}

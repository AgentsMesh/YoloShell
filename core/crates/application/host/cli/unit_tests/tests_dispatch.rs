const DISPATCH_INVALID_HOME_FILE: &str = "not-a-directory";
const DISPATCH_MISSING_SOCKET: &str = "missing.sock";

#[tokio::test]
async fn command_dispatch_rejects_invalid_serve_bridge_and_worker_inputs() {
    let root = tempfile::tempdir().unwrap();
    let invalid_home = root.path().join(DISPATCH_INVALID_HOME_FILE);
    std::fs::write(&invalid_home, b"").unwrap();
    assert!(
        run_command(Command::Serve {
            home: invalid_home.clone(),
            socket: None,
            detached_session: false,
            startup_guard: false,
        })
        .await
        .is_err()
    );
    assert!(
        run_command(Command::Bridge {
            stdio: true,
            home: invalid_home,
            socket: None,
        })
        .await
        .is_err()
    );
    assert!(
        run_command(Command::SessionWorker {
            home: root.path().to_path_buf(),
            terminal_id: uuid::Uuid::new_v4(),
        })
        .await
        .is_err()
    );
}

#[tokio::test]
async fn command_dispatch_handles_closed_catalog_input_and_missing_bootstrap_spec() {
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join(DISPATCH_MISSING_SOCKET);
    let parsed = Cli::try_parse_from([
        yolo_launch_contract::EXECUTABLE_NAME,
        cli::CATALOG_STREAM_COMMAND,
        cli::SOCKET_ARGUMENT,
        socket.to_str().unwrap(),
        cli::EXISTING_SOCKET_ARGUMENT,
    ])
    .unwrap();
    assert!(run_command(parsed.command).await.is_ok());
    assert!(run_command(Command::BootstrapChild).await.is_err());
}

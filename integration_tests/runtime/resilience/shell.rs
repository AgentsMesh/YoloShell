use super::*;

const BRIDGE_REQUEST_ID: &str = "bridge-request";
const HOST_IDENTITY_CAPTURE_CONTEXT: &str = "capture bootstrapped Host identity";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tilde_cwd_is_expanded_by_the_host_not_the_client() {
    let harness = Harness::start().await;
    let created = harness
        .client()
        .create_session(&CreateTerminal {
            creation_token: Uuid::new_v4(),
            name: Some("host-home".into()),
            cwd: PathBuf::from("~"),
            cols: None,
            rows: None,
            initial_input: None,
            context: None,
        })
        .await
        .unwrap();
    let view = harness
        .client()
        .get_session(created.terminal_id)
        .await
        .unwrap();
    harness.track_view(&view);
    let expected = PathBuf::from(std::env::var_os("HOME").unwrap())
        .canonicalize()
        .unwrap();
    assert_eq!(view.manifest.cwd, expected);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bridge_bootstraps_a_remote_host_when_no_server_is_running() {
    let home = tempfile::tempdir().unwrap();
    let mut bridge = ChildGuard::new(
        Command::new(runtime_binary())
            .args(yolo_launch_contract::ProcessRole::Host.arguments())
            .args([cli::BRIDGE_COMMAND, cli::STDIO_ARGUMENT, cli::HOME_ARGUMENT])
            .arg(home.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let request = RequestEnvelope::new(
        BRIDGE_REQUEST_ID,
        method::HOST_STATUS,
        yolo_capnp_value::Value::Null,
    );
    let response = bridge_request(&mut bridge, &request).await;
    assert!(response.ok);
    let status: yolo_runtime::HostStatus = response.decode_payload().unwrap();
    let host = ProcessIdentity::capture(status.host_pid).expect(HOST_IDENTITY_CAPTURE_CONTEXT);
    assert!(bridge.wait().unwrap().success());
    let client = Client::new(Paths::new(home.path()).host_socket().unwrap());
    assert_eq!(client.status().await.unwrap().host_pid, status.host_pid);
    assert!(host.signal(libc::SIGINT).unwrap());
    wait_for_identity_gone(host).await;
}

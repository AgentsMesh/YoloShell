use super::*;

const SESSION_EXIT_TIMEOUT: Duration = Duration::from_secs(5);
const SESSION_EXIT_POLL_INTERVAL: Duration = Duration::from_millis(20);
const COVERAGE_PROFILE_CLEANUP_DETAIL: &str = "discard deliberately aborted test process profile";
#[path = "cleanup/cleanup_peer.rs"]
mod cleanup_peer;
#[path = "cleanup/cleanup_rpc.rs"]
mod cleanup_rpc;
#[path = "cleanup/cleanup_timeout.rs"]
mod cleanup_timeout;

pub(crate) fn runtime_binary() -> PathBuf {
    let path = std::env::var_os(TEST_RUNTIME_ENVIRONMENT)
        .map(PathBuf::from)
        .or_else(|| bazel_runfile(HOST_BINARY_RUNFILE))
        .unwrap_or_else(|| panic!("{TEST_RUNTIME_MISSING}"));
    resolve_runtime_binary(path)
}

fn resolve_runtime_binary(path: PathBuf) -> PathBuf {
    path.canonicalize()
        .unwrap_or_else(|error| panic!("{TEST_RUNTIME_RESOLUTION_ERROR}: {error}"))
}

pub(crate) fn bazel_runfile(relative: &str) -> Option<PathBuf> {
    for root_variable in RUNFILE_ROOT_ENVIRONMENTS {
        if let Some(root) = std::env::var_os(root_variable) {
            for repository in RUNFILE_WORKSPACES {
                let candidate = PathBuf::from(&root).join(repository).join(relative);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

pub(crate) fn discard_incomplete_coverage_profile(pid: u32) {
    yolo_coverage_test_support::discard_aborted_process_profile(pid)
        .expect(COVERAGE_PROFILE_CLEANUP_DETAIL);
}

pub(crate) fn request_worker_termination(record: &RuntimeRecord) -> bool {
    let Ok(mut stream) = cleanup_connect::connect(&record.socket_path, CLEANUP_CONNECT_TIMEOUT)
    else {
        return false;
    };
    if cleanup_timeout::configure(&stream).is_err() {
        return false;
    }
    if !cleanup_peer::matches_runtime(&stream, record) {
        return false;
    }
    let auth = RequestEnvelope::new(
        CLEANUP_AUTH_REQUEST_ID,
        WORKER_AUTH_METHOD,
        to_value(&WorkerAuth {
            token: record.auth_token.clone(),
            controller_generation: record.controller_generation,
        })
        .unwrap(),
    );
    if write_frame_blocking(&mut stream, &auth).is_err() {
        return false;
    }
    let Ok(authenticated) = read_frame_blocking::<_, ResponseEnvelope>(&mut stream) else {
        return false;
    };
    if yolo_runtime::protocol::validate_response(&auth, &authenticated).is_err()
        || !authenticated.ok
    {
        return false;
    }
    let Ok(authenticated) = authenticated.decode_payload::<yolo_runtime::model::WorkerHello>()
    else {
        return false;
    };
    if !cleanup_peer::matches_runtime(&stream, record) || authenticated.runtime != *record {
        return false;
    }
    let terminate = RequestEnvelope::new(
        CLEANUP_TERMINATE_REQUEST_ID,
        method::TERMINATE_RUNTIME_EXACT,
        to_value(&FencedTerminateRequest {
            terminal_id: record.terminal_id,
            runtime_epoch: record.runtime_epoch,
            expected_status: None,
            force: false,
        })
        .unwrap(),
    );
    if !cleanup_peer::matches_runtime(&stream, record) {
        return false;
    }
    let Some(response) = cleanup_rpc::exchange(stream, terminate.clone()) else {
        return false;
    };
    response.ok && yolo_runtime::protocol::validate_response(&terminate, &response).is_ok()
}

pub(crate) fn spawn_host(home: &Path) -> SpawnedHost {
    prepare_host_home(home);
    let codex_source = home.join("test-codex-source");
    std::fs::create_dir_all(&codex_source).unwrap();
    if !codex_source.join("config.toml").exists() {
        std::fs::write(codex_source.join("config.toml"), b"model = 'fixture'\n").unwrap();
    }
    let mut command = Command::new(runtime_binary());
    command
        .args(yolo_launch_contract::ProcessRole::Host.arguments())
        .args([cli::SERVE_COMMAND, cli::HOME_ARGUMENT])
        .arg(home)
        .env(CODEX_HOME_ENV, codex_source)
        .env(SHELL_ENV, TEST_SHELL)
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    spawn_logged_host(home, &mut command)
}

pub(crate) async fn wait_for_exit(client: &Client, terminal_id: Uuid) {
    let deadline = tokio::time::Instant::now() + SESSION_EXIT_TIMEOUT;
    loop {
        if client
            .get_session(terminal_id)
            .await
            .unwrap()
            .runtime
            .as_ref()
            .is_some_and(|runtime| runtime.status == TerminalRuntimeStatus::Exited)
        {
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "session did not exit"
        );
        tokio::time::sleep(SESSION_EXIT_POLL_INTERVAL).await;
    }
}

#[cfg(test)]
#[path = "runtime_binary_tests.rs"]
mod runtime_binary_tests;

pub(super) fn release_root_first_trigger(marker: &Path) {
    let trigger = marker.with_extension(ROOT_FIRST_TRIGGER_EXTENSION);
    std::fs::write(trigger, EMPTY_MARKER).unwrap();
}

pub(super) async fn wait_for_marker_file(path: &Path, status: &Path, state: &WorkerState, timeout: Duration) {
    let result = tokio::time::timeout(timeout, async {
        while !path.exists() {
            tokio::time::sleep(PROCESS_DISCOVERY_POLL_INTERVAL).await;
        }
    })
    .await;
    result.unwrap_or_else(|_| {
        let stage = std::fs::read_to_string(status).unwrap_or_default();
        let output = lock_recover(&state.terminal).screen().contents();
        panic!(
            "{ROOT_FIRST_READY_TIMEOUT_DETAIL}; last stage: {stage}; \
             {ROOT_FIRST_TERMINAL_DETAIL}: {output}"
        )
    });
}

pub(super) async fn wait_for_process_file(
    path: &Path,
    root_pid: u32,
    timeout: Duration,
) -> process::ProcessIdentity {
    let result = tokio::time::timeout(timeout, async {
        loop {
            if let Ok(value) = std::fs::read_to_string(path)
                && let Ok(pid) = value.trim().parse()
                && let Ok(identity) = process::ProcessIdentity::capture(pid)
            {
                return identity;
            }
            tokio::time::sleep(PROCESS_DISCOVERY_POLL_INTERVAL).await;
        }
    })
    .await;
    result.unwrap_or_else(|_| {
        let stage = std::fs::read_to_string(path).unwrap_or_default();
        panic!("{ROOT_FIRST_PROCESS_TIMEOUT_DETAIL}; fixture root: {root_pid}; last stage: {stage}")
    })
}

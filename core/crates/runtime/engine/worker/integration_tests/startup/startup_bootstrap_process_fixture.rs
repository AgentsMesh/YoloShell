const SURVIVOR_HELPER_TEST: &str =
    "worker::tests::startup_bootstrap::bootstrap_survivor_process_helper";
const SURVIVOR_HELPER_ROLE_ENV: &str = "TEST_BOOTSTRAP_SURVIVOR_ROLE";
const SURVIVOR_HELPER_ROLE: &str = "hold-bootstrap-survivor";
const SURVIVOR_HELPER_MARKER_ENV: &str = "TEST_BOOTSTRAP_SURVIVOR_MARKER";
const SURVIVOR_HELPER_EXIT_MARKER_ENV: &str = "TEST_BOOTSTRAP_SURVIVOR_EXIT_MARKER";
const SURVIVOR_READY_LINE: &[u8] = b"survivor-ready\n";
const SURVIVOR_RELEASE_ACK_LINE: &[u8] = b"survivor-release-ack\n";
const SURVIVOR_RELEASE_COMMAND: &str = "release";
const SURVIVOR_TERMINAL_LINE_ENDINGS: &[char] = &['\r', '\n'];
const SURVIVOR_GONE_CONFIRMATIONS: usize = 3;
const SURVIVOR_EXIT_TIMEOUT_DETAIL: &str = "exact survivor child did not remain gone";

struct SurvivorWorkerTask(Option<tokio::task::JoinHandle<anyhow::Result<()>>>);

impl SurvivorWorkerTask {
    fn spawn(paths: Paths, terminal_id: Uuid) -> Self {
        Self(Some(tokio::spawn(run(paths, terminal_id))))
    }

    fn is_finished(&self) -> bool {
        self.0
            .as_ref()
            .is_none_or(tokio::task::JoinHandle::is_finished)
    }

    async fn finish(mut self) {
        self.0.take().unwrap().await.unwrap().unwrap();
    }
}

impl Drop for SurvivorWorkerTask {
    fn drop(&mut self) {
        if let Some(task) = self.0.as_ref()
            && !task.is_finished()
        {
            task.abort();
        }
    }
}

fn configure_survivor_launch(launch: &mut WorkerLaunchSpec, marker: &Path, exit_marker: &Path) {
    launch.command = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    launch.args = vec![
        crate::constants::TEST_HARNESS_EXACT_ARGUMENT.into(),
        SURVIVOR_HELPER_TEST.into(),
        crate::constants::TEST_HARNESS_NOCAPTURE_ARGUMENT.into(),
        crate::constants::TEST_HARNESS_SINGLE_THREAD_ARGUMENT.into(),
    ];
    launch
        .environment
        .insert(SURVIVOR_HELPER_ROLE_ENV.into(), SURVIVOR_HELPER_ROLE.into());
    launch.environment.insert(
        SURVIVOR_HELPER_MARKER_ENV.into(),
        marker.to_string_lossy().into_owned(),
    );
    launch.environment.insert(
        SURVIVOR_HELPER_EXIT_MARKER_ENV.into(),
        exit_marker.to_string_lossy().into_owned(),
    );
    launch.initial_input = Some(Vec::new());
}

pub(super) async fn wait_for_process_exit(identity: process::ProcessIdentity) {
    let mut gone = usize::default();
    let mut observed = process::ProcessIdentityObservation::Unknown;
    tokio::time::timeout(WORKER_TERMINATION_TIMEOUT, async {
        while gone < SURVIVOR_GONE_CONFIRMATIONS {
            observed = identity.observe();
            gone = match observed {
                process::ProcessIdentityObservation::Gone => gone.saturating_add(1),
                process::ProcessIdentityObservation::Matching
                | process::ProcessIdentityObservation::Unknown => usize::default(),
            };
            tokio::time::sleep(TEST_READY_POLL_INTERVAL).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("{SURVIVOR_EXIT_TIMEOUT_DETAIL}: {observed:?}"));
}

fn attachment_contains(attached: &AttachResult, expected: &[u8]) -> bool {
    attached
        .chunks
        .iter()
        .any(|chunk| bytes_contain(&chunk.data, expected))
        || attached
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| bytes_contain(&snapshot.data, expected))
}

fn bytes_contain(data: &[u8], expected: &[u8]) -> bool {
    data.windows(expected.len())
        .any(|window| window == expected)
}

#[test]
fn bootstrap_survivor_process_helper() {
    if std::env::var(SURVIVOR_HELPER_ROLE_ENV).as_deref() != Ok(SURVIVOR_HELPER_ROLE) {
        return;
    }
    use std::io::Write;
    let marker = std::env::var_os(SURVIVOR_HELPER_MARKER_ENV).unwrap();
    let exit_marker = std::env::var_os(SURVIVOR_HELPER_EXIT_MARKER_ENV).unwrap();
    let mut output = std::io::stdout().lock();
    output.write_all(SURVIVOR_READY_LINE).unwrap();
    output.flush().unwrap();
    std::fs::write(marker, TEST_MARKER_CONTENT).unwrap();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    assert_eq!(
        input.trim_end_matches(SURVIVOR_TERMINAL_LINE_ENDINGS),
        SURVIVOR_RELEASE_COMMAND
    );
    std::fs::write(exit_marker, TEST_MARKER_CONTENT).unwrap();
    output.write_all(SURVIVOR_RELEASE_ACK_LINE).unwrap();
    output.flush().unwrap();
}

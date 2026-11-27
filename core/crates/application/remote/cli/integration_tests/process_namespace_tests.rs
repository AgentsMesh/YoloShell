use super::process_handoff_tests::{BrokerProcess, ProcessFixture};
use super::{BrokerClient, BrokerPaths, fixture};
use std::process::{Command, Stdio};

const DISPLACED_STATE_DIRECTORY: &str = "state-displaced";
const SENTINEL_FILE: &str = "replacement-sentinel";
const SENTINEL_BYTES: &[u8] = b"preserve-replacement";
const SIGNAL_EXECUTABLE: &str = "/bin/kill";
const STOP_SIGNAL: &str = "-STOP";
const CONTINUE_SIGNAL: &str = "-CONT";

fn binary() -> std::path::PathBuf {
    std::fs::canonicalize(std::env::var_os(fixture::REMOTE_BINARY_ENVIRONMENT).unwrap()).unwrap()
}

fn signal(process: &mut BrokerProcess, signal: &str) {
    let pid = process.child_id().to_string();
    let status = Command::new(SIGNAL_EXECUTABLE)
        .arg(signal)
        .arg(pid)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
}

#[tokio::test]
async fn process_namespace_replacement_is_single_owner_and_recoverable() {
    let fixture = ProcessFixture::new().await;
    let binary = binary();
    let state = fixture.paths.state_dir().to_owned();
    let mut owner = BrokerProcess::start(&binary, &fixture.config, &state, &fixture.revision).await;
    fixture.wait_ready().await;
    signal(&mut owner, STOP_SIGNAL);

    let displaced = state.parent().unwrap().join(DISPLACED_STATE_DIRECTORY);
    std::fs::rename(&state, displaced).unwrap();
    let replacement = BrokerPaths::new(&state).unwrap();
    let sentinel = replacement.state_dir().join(SENTINEL_FILE);
    std::fs::write(&sentinel, SENTINEL_BYTES).unwrap();
    let mut contender =
        BrokerProcess::start(&binary, &fixture.config, &state, &fixture.revision).await;
    contender.wait_for_exit().await;
    assert!(!contender.is_running());
    assert_eq!(std::fs::read(&sentinel).unwrap(), SENTINEL_BYTES);

    signal(&mut owner, CONTINUE_SIGNAL);
    owner.wait_for_exit().await;
    let mut recovered =
        BrokerProcess::start(&binary, &fixture.config, &state, &fixture.revision).await;
    fixture.wait_ready().await;
    assert_eq!(std::fs::read(&sentinel).unwrap(), SENTINEL_BYTES);
    BrokerClient::new(replacement.management_socket())
        .shutdown(fixture.revision.clone())
        .await
        .unwrap();
    recovered.wait_for_exit().await;
}

#[tokio::test]
async fn sigkill_releases_external_instance_lock_for_recovery() {
    let fixture = ProcessFixture::new().await;
    let binary = binary();
    let state = fixture.paths.state_dir();
    let mut owner = BrokerProcess::start(&binary, &fixture.config, state, &fixture.revision).await;
    fixture.wait_ready().await;
    owner.kill_and_wait();

    let mut recovered =
        BrokerProcess::start(&binary, &fixture.config, state, &fixture.revision).await;
    fixture.wait_ready().await;
    BrokerClient::new(fixture.paths.management_socket())
        .shutdown(fixture.revision.clone())
        .await
        .unwrap();
    recovered.wait_for_exit().await;
}

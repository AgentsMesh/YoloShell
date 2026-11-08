use super::tests::Fixture;
use super::*;
use crate::fixture_constants as fixture;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use yolo_process::{ProcessIdentity, ProcessIdentityObservation};

const HELPER_FILTER: &str = "scavenge::family_tests::orphan_process_family_helper";
const ROLE_ENVIRONMENT: &str = "YOLO_SCAVENGE_FAMILY_ROLE";
const ROOT_READY_ENVIRONMENT: &str = "YOLO_SCAVENGE_ROOT_READY";
const PLAIN_PID_ENVIRONMENT: &str = "YOLO_SCAVENGE_PLAIN_PID";
const SETSID_PID_ENVIRONMENT: &str = "YOLO_SCAVENGE_SETSID_PID";
const ROOT_ROLE: &str = "root";
const PLAIN_ROLE: &str = "plain";
const SETSID_ROLE: &str = "setsid";
const ROOT_READY_FILE: &str = "root.ready";
const PLAIN_PID_FILE: &str = "plain.pid";
const SETSID_PID_FILE: &str = "setsid.pid";
const HELPER_LIFETIME: Duration = Duration::from_secs(30);
const OBSERVATION_TIMEOUT: Duration = Duration::from_secs(5);
const OBSERVATION_INTERVAL: Duration = Duration::from_millis(10);
const SETSID_FAILURE: libc::pid_t = -1;
const EMPTY_BYTES: &[u8] = b"";

#[test]
fn orphan_broker_family_contains_plain_and_setsid_descendants() {
    let fixture = Fixture::new();
    let helper_root = tempfile::tempdir().unwrap();
    let root_ready = helper_root.path().join(ROOT_READY_FILE);
    let plain_pid = helper_root.path().join(PLAIN_PID_FILE);
    let setsid_pid = helper_root.path().join(SETSID_PID_FILE);
    let authority = fixture.family.authority.to_string();
    let mut broker = spawn_helper(ROOT_ROLE, &root_ready, &plain_pid, &setsid_pid, &authority);
    wait_for_path(&root_ready);
    let broker_identity = ProcessIdentity::capture(broker.id()).unwrap();
    let descendants = [read_identity(&plain_pid), read_identity(&setsid_pid)];
    let mut record = fixture.intent(exited_identity());
    record.activate(broker_identity).unwrap();
    fixture
        .store
        .write(&fixture.family.record, &record)
        .unwrap();

    run(&fixture.store).unwrap();

    broker.wait().unwrap();
    wait_until_gone(broker_identity);
    for descendant in descendants {
        wait_until_gone(descendant);
    }
    assert!(fixture.record_missing());
}

#[test]
fn orphan_process_family_helper() {
    match std::env::var(ROLE_ENVIRONMENT).as_deref() {
        Ok(ROOT_ROLE) => run_root_helper(),
        Ok(PLAIN_ROLE) => run_leaf_helper(false, PLAIN_PID_ENVIRONMENT),
        Ok(SETSID_ROLE) => run_leaf_helper(true, SETSID_PID_ENVIRONMENT),
        _ => {}
    }
}

fn run_root_helper() {
    assert_ne!(unsafe { libc::setsid() }, SETSID_FAILURE);
    let root_ready = required_path(ROOT_READY_ENVIRONMENT);
    let plain_pid = required_path(PLAIN_PID_ENVIRONMENT);
    let setsid_pid = required_path(SETSID_PID_ENVIRONMENT);
    let authority = std::env::var(fixture::BROKER_SUPERVISION_AUTHORITY_ENVIRONMENT).unwrap();
    let _plain = spawn_helper(PLAIN_ROLE, &root_ready, &plain_pid, &setsid_pid, &authority);
    let _setsid = spawn_helper(
        SETSID_ROLE,
        &root_ready,
        &plain_pid,
        &setsid_pid,
        &authority,
    );
    wait_for_path(&plain_pid);
    wait_for_path(&setsid_pid);
    std::fs::write(root_ready, EMPTY_BYTES).unwrap();
    std::thread::sleep(HELPER_LIFETIME);
}

fn run_leaf_helper(new_session: bool, pid_environment: &str) {
    if new_session {
        assert_ne!(unsafe { libc::setsid() }, SETSID_FAILURE);
    }
    std::fs::write(
        required_path(pid_environment),
        std::process::id().to_string(),
    )
    .unwrap();
    std::thread::sleep(HELPER_LIFETIME);
}

fn spawn_helper(role: &str, ready: &Path, plain: &Path, setsid: &Path, authority: &str) -> Child {
    Command::new(std::env::current_exe().unwrap())
        .args([
            fixture::TEST_HARNESS_EXACT_ARGUMENT,
            HELPER_FILTER,
            fixture::TEST_HARNESS_NOCAPTURE_ARGUMENT,
        ])
        .env(ROLE_ENVIRONMENT, role)
        .env(ROOT_READY_ENVIRONMENT, ready)
        .env(PLAIN_PID_ENVIRONMENT, plain)
        .env(SETSID_PID_ENVIRONMENT, setsid)
        .env(fixture::BROKER_SUPERVISION_AUTHORITY_ENVIRONMENT, authority)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

fn wait_for_path(path: &Path) {
    let deadline = Instant::now() + OBSERVATION_TIMEOUT;
    while !path.exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(OBSERVATION_INTERVAL);
    }
}

fn read_identity(path: &Path) -> ProcessIdentity {
    let process_id = std::fs::read_to_string(path).unwrap().parse().unwrap();
    ProcessIdentity::capture(process_id).unwrap()
}

fn wait_until_gone(identity: ProcessIdentity) {
    let deadline = Instant::now() + OBSERVATION_TIMEOUT;
    while identity.observe() != ProcessIdentityObservation::Gone {
        assert!(Instant::now() < deadline);
        std::thread::sleep(OBSERVATION_INTERVAL);
    }
}

fn required_path(environment: &str) -> PathBuf {
    std::env::var_os(environment).map(PathBuf::from).unwrap()
}

fn exited_identity() -> ProcessIdentity {
    let mut child = Command::new(fixture::TEST_EXIT_EXECUTABLE).spawn().unwrap();
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    child.wait().unwrap();
    identity
}

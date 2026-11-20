use super::*;
use std::os::unix::fs::PermissionsExt;
use yolo_process_test_support::ProcessIdentityObservation;

const HELPER_ROLE_ENV: &str = "YOLO_POWER_LOSS_ESCAPE_ROLE";
const HELPER_READY_ENV: &str = "YOLO_POWER_LOSS_ESCAPE_READY";
const HELPER_PID_ENV: &str = "YOLO_POWER_LOSS_ESCAPE_PID";
const ROOT_ROLE: &str = "root";
const INTERMEDIATE_ROLE: &str = "intermediate";
const LEAF_ROLE: &str = "leaf";
const WRAPPER_NAME: &str = "pre-capture-escape.sh";
const READY_NAME: &str = "pre-capture-escape.ready";
const PID_NAME: &str = "pre-capture-escape.pid";
const SESSION_NAME: &str = "pre-capture-escaped-holder";
const WRITER_HOLDER: &str = "pre-capture-escape-test";
const SCRIPT_MODE: u32 = 0o700;
const HELPER_LIFETIME: Duration = Duration::from_secs(60);
const HELPER_TIMEOUT_DETAIL: &str = "pre-capture escaped PTY holder did not become ready";
const HELPER_GONE_DETAIL: &str = "pre-capture escaped PTY holder survived simulated power loss";
const SETSID_FAILURE: libc::pid_t = -1;

struct EscapedHolderCleanup(ProcessIdentity);

impl Drop for EscapedHolderCleanup {
    fn drop(&mut self) {
        if self.0.observe() != ProcessIdentityObservation::Gone
            && matches!(self.0.signal(libc::SIGKILL), Ok(true))
        {
            let deadline = Instant::now() + PROCESS_EXIT_TIMEOUT;
            while self.0.observe() != ProcessIdentityObservation::Gone && Instant::now() < deadline
            {
                std::thread::sleep(PROCESS_POLL_INTERVAL);
            }
            if self.0.observe() == ProcessIdentityObservation::Gone {
                discard_incomplete_coverage_profile(self.0.pid);
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn power_loss_captures_a_preexisting_reparented_pty_holder() {
    let mut harness = Harness::start().await;
    let session = harness.create_cat(SESSION_NAME).await;
    let terminal_id = session.manifest.terminal_id;
    let paths = harness.paths();
    let record: RuntimeRecord = read_json(&paths.runtime_record(terminal_id)).unwrap();
    let ready = harness.home.path().join(READY_NAME);
    let pid = harness.home.path().join(PID_NAME);
    install_wrapper(&harness, &ready, &pid);
    launch_wrapper(&harness, &record).await;
    wait_for_path(&ready).await;
    let escaped = read_identity(&pid);
    let _cleanup = EscapedHolderCleanup(escaped);
    assert_eq!(
        unsafe { libc::getpgid(escaped.pid as i32) },
        escaped.pid as i32
    );

    simulate_power_loss(&mut harness, std::slice::from_ref(&record)).await;

    assert_eq!(
        escaped.observe(),
        ProcessIdentityObservation::Gone,
        "{HELPER_GONE_DETAIL}: {escaped:?}"
    );
}

#[test]
fn pre_capture_escaped_pty_holder_helper() {
    match std::env::var(HELPER_ROLE_ENV).as_deref() {
        Ok(ROOT_ROLE) => spawn_and_wait(INTERMEDIATE_ROLE),
        Ok(INTERMEDIATE_ROLE) => spawn_detached_role(LEAF_ROLE),
        Ok(LEAF_ROLE) => run_leaf(),
        _ => {}
    }
}

fn install_wrapper(harness: &Harness, ready: &Path, pid: &Path) {
    let executable = std::env::current_exe().unwrap();
    let script = format!(
        "#!/bin/sh\n{HELPER_ROLE_ENV}={ROOT_ROLE} {HELPER_READY_ENV}={} {HELPER_PID_ENV}={} {} {POWER_LOSS_ESCAPE_HELPER_FILTER} {TEST_HARNESS_NOCAPTURE_ARGUMENT}\n",
        shell_quote(ready),
        shell_quote(pid),
        shell_quote(&executable),
    );
    let path = harness.home.path().join(WRAPPER_NAME);
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(SCRIPT_MODE)).unwrap();
}

async fn launch_wrapper(harness: &Harness, record: &RuntimeRecord) {
    let target = runtime_ref_from_record(record);
    let client = harness.client();
    let lease = client
        .acquire_writer(target, WRITER_HOLDER, None)
        .await
        .unwrap();
    client
        .input(
            target,
            lease.generation,
            &lease.writer_capability,
            format!("./{WRAPPER_NAME}\n").into_bytes(),
        )
        .await
        .unwrap();
}

fn spawn_and_wait(role: &str) {
    let mut child = helper_command(role).spawn().unwrap();
    assert!(child.wait().unwrap().success());
}

fn spawn_detached_role(role: &str) {
    drop(helper_command(role).spawn().unwrap());
}

fn helper_command(role: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .arg(POWER_LOSS_ESCAPE_HELPER_FILTER)
        .arg(TEST_HARNESS_NOCAPTURE_ARGUMENT)
        .env(HELPER_ROLE_ENV, role);
    command
}

fn run_leaf() {
    assert_ne!(unsafe { libc::setsid() }, SETSID_FAILURE);
    unsafe {
        libc::signal(libc::SIGHUP, libc::SIG_IGN);
        libc::signal(libc::SIGTERM, libc::SIG_IGN);
    }
    std::fs::write(
        required_path(HELPER_PID_ENV),
        std::process::id().to_string(),
    )
    .unwrap();
    std::fs::write(required_path(HELPER_READY_ENV), []).unwrap();
    std::thread::sleep(HELPER_LIFETIME);
}

async fn wait_for_path(path: &Path) {
    let deadline = tokio::time::Instant::now() + PROCESS_EXIT_TIMEOUT;
    while !path.exists() {
        assert!(
            tokio::time::Instant::now() < deadline,
            "{HELPER_TIMEOUT_DETAIL}: {}",
            path.display()
        );
        tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
    }
}

fn read_identity(path: &Path) -> ProcessIdentity {
    let pid = std::fs::read_to_string(path)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    ProcessIdentity::capture(pid).unwrap()
}

fn required_path(name: &str) -> PathBuf {
    std::env::var_os(name).map(PathBuf::from).unwrap()
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

use super::*;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::process::{Child, Command};
use std::time::Instant;

pub(super) fn spawn_rejection(fixture: &Fixture, digest: &str, expected_error: &str) {
    let mut child = helper_command(REJECT_MODE, fixture, digest)
        .env(EXPECTED_ERROR_ENV, expected_error)
        .spawn()
        .unwrap();
    wait_success(&mut child);
}

pub(super) fn helper_command(mode: &str, fixture: &Fixture, digest: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .arg(super::process_constants::AUTHORITY_GUARD_HELPER_FILTER)
        .arg(super::process_constants::TEST_HARNESS_NOCAPTURE_ARGUMENT)
        .env(HELPER_MODE_ENV, mode)
        .env(DOCUMENT_PATH_ENV, &fixture.document)
        .env(LOCK_PATH_ENV, &fixture.lock)
        .env(EXPECTED_DIGEST_ENV, digest);
    command
}

pub(super) fn committed_profile(profile: Value, digest: &str) -> Value {
    json!({
        (VERSION_FIELD): AUTHORITY_VERSION,
        (HOST_ID_FIELD): HOST_ID,
        (PHASE_FIELD): COMMITTED_PHASE,
        (PROFILE_FIELD): profile,
        (PROFILE_SHA256_FIELD): digest,
    })
}

pub(super) fn deleted() -> Value {
    json!({
        (VERSION_FIELD): AUTHORITY_VERSION,
        (HOST_ID_FIELD): HOST_ID,
        (PHASE_FIELD): DELETED_PHASE,
    })
}

pub(super) fn durable_replace(path: &Path, value: &Value) {
    let staged = path.with_extension(STAGED_EXTENSION);
    let bytes = serde_json::to_vec(value).unwrap();
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(PRIVATE_MODE)
        .open(&staged)
        .unwrap();
    file.write_all(&bytes).unwrap();
    file.set_permissions(std::fs::Permissions::from_mode(PRIVATE_MODE))
        .unwrap();
    file.sync_all().unwrap();
    std::fs::rename(staged, path).unwrap();
    File::open(path.parent().unwrap())
        .unwrap()
        .sync_all()
        .unwrap();
}

pub(super) fn sync_file(path: &Path) {
    OpenOptions::new()
        .read(true)
        .open(path)
        .unwrap()
        .sync_all()
        .unwrap();
}

pub(super) fn write_signal(path: &Path) {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(PRIVATE_MODE)
        .open(path)
        .unwrap();
    file.write_all(SIGNAL_BYTES).unwrap();
    file.set_permissions(std::fs::Permissions::from_mode(PRIVATE_MODE))
        .unwrap();
    file.sync_all().unwrap();
}

pub(super) fn wait_for_signal(path: &Path, child: &mut Child) {
    let deadline = Instant::now() + HELPER_TIMEOUT;
    while !path.exists() {
        assert!(
            child.try_wait().unwrap().is_none(),
            "authority helper exited early"
        );
        assert!(
            Instant::now() < deadline,
            "authority helper readiness timed out"
        );
        std::thread::sleep(HELPER_POLL_INTERVAL);
    }
}

pub(super) fn wait_success(child: &mut Child) {
    let deadline = Instant::now() + HELPER_TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "authority helper failed: {status}");
            return;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("authority helper exit timed out");
        }
        std::thread::sleep(HELPER_POLL_INTERVAL);
    }
}

pub(super) fn wait_until(path: &Path, timeout: Duration, message: &str) {
    let deadline = Instant::now() + timeout;
    while !path.exists() {
        assert!(Instant::now() < deadline, "{message}");
        std::thread::sleep(HELPER_POLL_INTERVAL);
    }
}

pub(super) fn required_environment(name: &str) -> std::ffi::OsString {
    std::env::var_os(name).unwrap_or_else(|| panic!("missing helper environment: {name}"))
}

pub(super) fn required_text_environment(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("invalid helper environment: {name}"))
}

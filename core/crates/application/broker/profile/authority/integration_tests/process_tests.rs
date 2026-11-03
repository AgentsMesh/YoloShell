use super::test_fixture::*;
use super::*;
use serde_json::{Value, json};
use std::fs::OpenOptions;
use std::os::fd::AsRawFd;
use std::time::Duration;
use yolo_wire::field::PHASE as PHASE_FIELD;

#[path = "../test_support/process_support.rs"]
mod support;
use support::*;
#[path = "../test_support/constants.rs"]
mod process_constants;

const HELPER_MODE_ENV: &str = "YOLO_AUTHORITY_HELPER_MODE";
const DOCUMENT_PATH_ENV: &str = "YOLO_AUTHORITY_DOCUMENT_PATH";
const LOCK_PATH_ENV: &str = "YOLO_AUTHORITY_LOCK_PATH";
const EXPECTED_DIGEST_ENV: &str = "YOLO_AUTHORITY_EXPECTED_DIGEST";
const EXPECTED_ERROR_ENV: &str = "YOLO_AUTHORITY_EXPECTED_ERROR";
const READY_PATH_ENV: &str = "YOLO_AUTHORITY_READY_PATH";
const RELEASE_PATH_ENV: &str = "YOLO_AUTHORITY_RELEASE_PATH";
const HOLD_MODE: &str = "hold-committed";
const REJECT_MODE: &str = "reject-start";
const READY_FILE_NAME: &str = "holder.ready";
const RELEASE_FILE_NAME: &str = "holder.release";
const STAGED_EXTENSION: &str = "authority-staged";
const SIGNAL_BYTES: &[u8] = b"authority-signal";
const DISPLAY_NAME_FIELD: &str = "displayName";
const VERSION_FIELD: &str = "version";
const HOST_ID_FIELD: &str = "hostID";
const PROFILE_FIELD: &str = "profile";
const PROFILE_SHA256_FIELD: &str = "profileSHA256";
const COMMITTED_PHASE: &str = "committed";
const DELETED_PHASE: &str = "deleted";
const PROFILE_C_DISPLAY_NAME: &str = "Replacement host";
const AUTHORITY_VERSION: u16 = 1;
const FLOCK_SUCCESS: libc::c_int = 0;
const NOT_COMMITTED_ERROR: &str = "authority is not committed";
const DIGEST_MISMATCH_ERROR: &str = "authority digest mismatch";
const HELPER_TIMEOUT: Duration = Duration::from_secs(5);
const HELPER_POLL_INTERVAL: Duration = Duration::from_millis(5);

#[test]
fn external_process_holds_the_committed_start_lock() {
    let fixture = Fixture::new(committed());
    durable_replace(&fixture.document, &committed());
    sync_file(&fixture.lock);
    let ready = fixture._root.path().join(READY_FILE_NAME);
    let release = fixture._root.path().join(RELEASE_FILE_NAME);
    let mut holder = helper_command(HOLD_MODE, &fixture, PROFILE_SHA256)
        .env(READY_PATH_ENV, &ready)
        .env(RELEASE_PATH_ENV, &release)
        .spawn()
        .unwrap();
    wait_for_signal(&ready, &mut holder);

    let contender = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&fixture.lock)
        .unwrap();
    let result = unsafe { libc::flock(contender.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    let failure = std::io::Error::last_os_error().raw_os_error();
    if result == FLOCK_SUCCESS {
        unsafe { libc::flock(contender.as_raw_fd(), libc::LOCK_UN) };
    }
    assert_ne!(
        result, FLOCK_SUCCESS,
        "external committed guard did not hold the lock"
    );
    assert!(
        failure == Some(libc::EWOULDBLOCK) || failure == Some(libc::EAGAIN),
        "external contender failed with an unexpected error: {failure:?}"
    );

    write_signal(&release);
    wait_success(&mut holder);
}

#[test]
fn external_start_rejects_blocked_replaced_and_deleted_authority() {
    let fixture = Fixture::new(committed());
    durable_replace(&fixture.document, &blocked(Uuid::new_v4()));
    spawn_rejection(&fixture, PROFILE_SHA256, NOT_COMMITTED_ERROR);

    let mut profile_c = full_profile();
    profile_c[DISPLAY_NAME_FIELD] = json!(PROFILE_C_DISPLAY_NAME);
    let digest_c = digest::profile_sha256(&profile_c).unwrap();
    durable_replace(&fixture.document, &committed_profile(profile_c, &digest_c));
    spawn_rejection(&fixture, PROFILE_SHA256, DIGEST_MISMATCH_ERROR);

    durable_replace(&fixture.document, &deleted());
    spawn_rejection(&fixture, PROFILE_SHA256, NOT_COMMITTED_ERROR);
}

#[test]
fn authority_guard_subprocess_helper() {
    let Some(mode) = std::env::var_os(HELPER_MODE_ENV) else {
        return;
    };
    let document = PathBuf::from(required_environment(DOCUMENT_PATH_ENV));
    let lock = PathBuf::from(required_environment(LOCK_PATH_ENV));
    let digest = required_text_environment(EXPECTED_DIGEST_ENV);
    match mode.to_str().unwrap() {
        HOLD_MODE => hold_guard(&document, &lock, &digest),
        REJECT_MODE => {
            let expected = required_text_environment(EXPECTED_ERROR_ENV);
            let error = StartAuthorityGuard::acquire(&document, &lock, &digest)
                .err()
                .expect("external stale start unexpectedly acquired authority");
            assert!(
                error.to_string().contains(&expected),
                "unexpected rejection: {error:#}"
            );
        }
        unexpected => panic!("unexpected authority helper mode: {unexpected}"),
    }
}

fn hold_guard(document: &Path, lock: &Path, digest: &str) {
    let guard = StartAuthorityGuard::acquire(document, lock, digest).unwrap();
    let ready = PathBuf::from(required_environment(READY_PATH_ENV));
    let release = PathBuf::from(required_environment(RELEASE_PATH_ENV));
    write_signal(&ready);
    wait_until(
        &release,
        HELPER_TIMEOUT,
        "authority helper release timed out",
    );
    drop(guard);
}

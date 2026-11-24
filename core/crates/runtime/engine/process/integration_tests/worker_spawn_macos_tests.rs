use super::spawn;
use std::ffi::OsString;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::Path;
use std::time::{Duration, Instant};

const SHELL: &str = "/bin/sh";
const PS: &str = "/bin/ps";
const TR: &str = "/usr/bin/tr";
const PROCESS_GROUP_FIELD: &str = "pgid";
const FD_DUPLICATION_MINIMUM: libc::c_int = 100;
const FD_PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const FD_PROBE_INTERVAL: Duration = Duration::from_millis(10);
const READY_MARKER: &[u8] = b"ready";
const ENVIRONMENT_PROBE_KEY: &str = "YOLO_WORKER_ENVIRONMENT_PROBE";
const ENVIRONMENT_PROBE_VALUE: &str = "worker value=preserved";
const INHERITED_ENVIRONMENT_KEY: &str = "USER";

fn session_probe_script() -> String {
    format!(
        "group=$({PS} -o {PROCESS_GROUP_FIELD}= -p $$ | {TR} -d ' '); if test \"$group\" = \"$$\" && test ! -e /dev/fd/$2; then printf ready > \"$1\"; fi",
    )
}

#[test]
fn raw_spawn_closes_lock_and_creates_an_independent_session() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join("ready");
    let (source, _peer) = std::os::unix::net::UnixStream::pair().unwrap();
    let descriptor =
        unsafe { libc::fcntl(source.as_raw_fd(), libc::F_DUPFD, FD_DUPLICATION_MINIMUM) };
    assert!(descriptor >= FD_DUPLICATION_MINIMUM);
    let descriptor = unsafe { OwnedFd::from_raw_fd(descriptor) };
    assert_eq!(
        unsafe { libc::fcntl(descriptor.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
        0
    );
    let arguments = vec![
        OsString::from("-c"),
        OsString::from(session_probe_script()),
        OsString::from("yolo-worker-spawn-probe"),
        OsString::from_vec(marker.as_os_str().as_bytes().to_vec()),
        OsString::from(descriptor.as_raw_fd().to_string()),
    ];

    let started = Instant::now();
    let pid = spawn(Path::new(SHELL), &arguments, &[], &[descriptor.as_raw_fd()]).unwrap();
    assert!(started.elapsed() < FD_PROBE_TIMEOUT);
    wait_for_exit(pid);
    assert_eq!(read_marker(&marker), READY_MARKER);
    assert!(unsafe { libc::fcntl(descriptor.as_raw_fd(), libc::F_GETFD) } >= 0);
}

#[test]
fn raw_spawn_applies_environment_changes_to_the_actual_child() {
    assert!(std::env::var_os(INHERITED_ENVIRONMENT_KEY).is_some());
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join("environment");
    let script = format!(
        "test -z \"${{{INHERITED_ENVIRONMENT_KEY}+x}}\" && printf '%s' \"${ENVIRONMENT_PROBE_KEY}\" > \"$1\""
    );
    let arguments = [
        OsString::from("-c"),
        OsString::from(script),
        OsString::from("yolo-worker-environment-probe"),
        marker.as_os_str().to_owned(),
    ];
    let environment = [
        (
            ENVIRONMENT_PROBE_KEY.into(),
            Some(ENVIRONMENT_PROBE_VALUE.into()),
        ),
        (INHERITED_ENVIRONMENT_KEY.into(), None),
    ];
    let pid = spawn(Path::new(SHELL), &arguments, &environment, &[]).unwrap();
    wait_for_exit(pid);
    assert_eq!(read_marker(&marker), ENVIRONMENT_PROBE_VALUE.as_bytes());
}

fn wait_for_exit(pid: libc::pid_t) {
    let deadline = Instant::now() + FD_PROBE_TIMEOUT;
    loop {
        let mut status = libc::c_int::default();
        let result = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        if result == pid {
            assert!(libc::WIFEXITED(status));
            assert_eq!(libc::WEXITSTATUS(status), 0);
            return;
        }
        assert_eq!(result, 0);
        assert!(Instant::now() < deadline);
        std::thread::sleep(FD_PROBE_INTERVAL);
    }
}

fn read_marker(path: &Path) -> Vec<u8> {
    let deadline = Instant::now() + FD_PROBE_TIMEOUT;
    loop {
        if let Ok(bytes) = std::fs::read(path) {
            return bytes;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(FD_PROBE_INTERVAL);
    }
}

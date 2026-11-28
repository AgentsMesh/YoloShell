use super::*;
use std::ffi::OsStr;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::time::Instant;

#[path = "macos_mapped_executable_process_support.rs"]
mod support;
use support::*;

#[test]
fn same_pid_retains_loaded_vnode_across_path_replacement() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(TARGET_FILE);
    let replacement = root.path().join(REPLACEMENT_FILE);
    let release = root.path().join(RELEASE_FILE);
    copy_executable(Path::new(ORIGINAL_EXECUTABLE), &target);
    let retained = std::fs::File::open(&target).unwrap();
    let expected = retained_identity(retained.as_raw_fd()).unwrap();
    let child = spawn_child(&target, &release);
    let pid = child.process_id();
    let live = capture_live_identity(pid);
    let observer = crate::process::MacosExecObserver::register(live).unwrap();
    std::fs::write(&release, EMPTY_SIGNAL).unwrap();
    assert_eq!(
        observer.wait_until(Instant::now() + WAIT_TIMEOUT).unwrap(),
        live
    );

    assert_eq!(
        verify_mapped_executable(pid, retained.as_raw_fd()).unwrap(),
        expected
    );
    copy_executable(Path::new(REPLACEMENT_EXECUTABLE), &replacement);
    std::fs::rename(&replacement, &target).unwrap();
    let replaced = std::fs::File::open(&target).unwrap();
    assert_ne!(retained_identity(replaced.as_raw_fd()).unwrap(), expected);
    assert!(verify_mapped_executable(pid, replaced.as_raw_fd()).is_err());
    assert_eq!(
        verify_mapped_executable(pid, retained.as_raw_fd()).unwrap(),
        expected
    );

    assert!(live.signal(libc::SIGCONT).unwrap());
    wait_for_clean_exit(child);
}

#[test]
fn suspended_setexec_helper() {
    if std::env::var_os(HELPER_ROLE_ENV).as_deref() != Some(OsStr::new(HELPER_ROLE)) {
        return;
    }
    let target = wait_for_release_and_target();
    let status = overlay_suspended(&target);
    panic!("suspended SETEXEC returned unexpectedly: {status}");
}

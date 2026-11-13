const ROOT_FIRST_HELPER_GATE_TIMEOUT: Duration = Duration::from_secs(10);
const ROOT_FIRST_HELPER_REAP_TIMEOUT: Duration = Duration::from_secs(3);
const ROOT_FIRST_INTERMEDIATE_LOCK_TIMEOUT_DETAIL: &str =
    "root-first intermediate lock acquisition timed out";
const ROOT_FIRST_LEAF_LOCK_TIMEOUT_DETAIL: &str = "root-first leaf lock acquisition timed out";
const ROOT_FIRST_INTERMEDIATE_RELEASE_TIMEOUT_DETAIL: &str =
    "root-first intermediate release gate timed out";
const ROOT_FIRST_REOPEN_RELEASE_TIMEOUT_DETAIL: &str =
    "root-first leaf reopen release gate timed out";
const ROOT_FIRST_LEAF_LAUNCH_TIMEOUT_DETAIL: &str = "root-first leaf launch gate timed out";
const ROOT_FIRST_LEAF_REAP_TIMEOUT_DETAIL: &str = "root-first leaf reap timed out";

struct RootFirstLeafChild(Option<std::process::Child>);

impl RootFirstLeafChild {
    fn new(child: std::process::Child) -> Self {
        Self(Some(child))
    }

    fn child_mut(&mut self) -> &mut std::process::Child {
        self.0.as_mut().unwrap()
    }
}

impl Drop for RootFirstLeafChild {
    fn drop(&mut self) {
        let Some(mut child) = self.0.take() else {
            return;
        };
        if child.try_wait().ok().flatten().is_some() {
            return;
        }
        let _ = child.kill();
        let deadline = Instant::now() + ROOT_FIRST_HELPER_REAP_TIMEOUT;
        while Instant::now() < deadline {
            if child.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
        }
    }
}

fn open_intermediate_lock(path: &Path, mode: IntermediateLockMode) -> std::fs::File {
    use std::os::unix::fs::OpenOptionsExt;
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).custom_flags(libc::O_CLOEXEC);
    if matches!(mode, IntermediateLockMode::Create) {
        options.create(true);
    }
    options.open(path).unwrap()
}

fn lock_exclusive(file: &std::fs::File, timeout_detail: &'static str) {
    use std::os::fd::AsRawFd;
    let deadline = Instant::now() + ROOT_FIRST_HELPER_GATE_TIMEOUT;
    loop {
        let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if result == ROOT_FIRST_LOCK_SUCCESS {
            return;
        }
        let error = std::io::Error::last_os_error();
        assert_eq!(error.raw_os_error(), Some(libc::EWOULDBLOCK));
        assert!(Instant::now() < deadline, "{timeout_detail}");
        std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
    }
}

fn wait_for_helper_path(path: &Path, timeout_detail: &'static str) {
    let deadline = Instant::now() + ROOT_FIRST_HELPER_GATE_TIMEOUT;
    while !path.exists() {
        assert!(Instant::now() < deadline, "{timeout_detail}");
        std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
    }
}

fn wait_for_leaf_launch(path: &Path, leaf: &mut std::process::Child, marker: &Path) {
    let deadline = Instant::now() + ROOT_FIRST_HELPER_GATE_TIMEOUT;
    while !path.exists() {
        if let Some(status) = leaf.try_wait().unwrap() {
            panic_leaf_launch_failure(leaf, marker, status);
        }
        if Instant::now() >= deadline {
            leaf.kill().unwrap();
            let status = wait_for_leaf_reap(leaf);
            std::fs::write(marker, ROOT_FIRST_LEAF_LAUNCH_TIMEOUT_DETAIL).unwrap();
            panic_leaf_launch_failure(leaf, marker, status);
        }
        std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
    }
}

fn wait_for_leaf_reap(leaf: &mut std::process::Child) -> std::process::ExitStatus {
    let deadline = Instant::now() + ROOT_FIRST_HELPER_REAP_TIMEOUT;
    loop {
        if let Some(status) = leaf.try_wait().unwrap() {
            return status;
        }
        assert!(
            Instant::now() < deadline,
            "{ROOT_FIRST_LEAF_REAP_TIMEOUT_DETAIL}"
        );
        std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
    }
}

fn panic_leaf_launch_failure(
    leaf: &mut std::process::Child,
    marker: &Path,
    status: std::process::ExitStatus,
) -> ! {
    use std::io::Read;
    let previous = std::fs::read_to_string(marker).unwrap_or_default();
    let mut diagnostic = String::new();
    leaf.stderr
        .take()
        .unwrap()
        .read_to_string(&mut diagnostic)
        .unwrap();
    let stage = format!(
        "{ROOT_FIRST_STAGE_LEAF_EXITED_EARLY}{ROOT_FIRST_STAGE_STATUS_SEPARATOR}{status}\
         {ROOT_FIRST_STAGE_FIELD_SEPARATOR}{ROOT_FIRST_STAGE_PREVIOUS_LABEL}\
         {ROOT_FIRST_STAGE_STATUS_SEPARATOR}{previous}{ROOT_FIRST_STAGE_FIELD_SEPARATOR}\
         {ROOT_FIRST_STAGE_STDERR_LABEL}{ROOT_FIRST_STAGE_STATUS_SEPARATOR}{diagnostic}"
    );
    std::fs::write(marker, stage).unwrap();
    panic!("{ROOT_FIRST_LEAF_EARLY_EXIT_DETAIL}: {status}");
}

fn exit_root_first_helper() -> ! {
    unsafe { libc::_exit(ROOT_FIRST_HELPER_SUCCESS_EXIT_CODE) }
}

fn ignore_root_first_control_signals() {
    unsafe {
        libc::signal(libc::SIGHUP, libc::SIG_IGN);
        libc::signal(libc::SIGTERM, libc::SIG_IGN);
    }
}

fn root_first_terminal_path() -> std::path::PathBuf {
    use std::os::unix::ffi::OsStringExt;
    let mut path = [0 as libc::c_char; libc::PATH_MAX as usize];
    let result = unsafe { libc::ttyname_r(libc::STDOUT_FILENO, path.as_mut_ptr(), path.len()) };
    assert_eq!(result, ROOT_FIRST_TTY_NAME_SUCCESS);
    let bytes = unsafe { std::ffi::CStr::from_ptr(path.as_ptr()) }
        .to_bytes()
        .to_vec();
    std::path::PathBuf::from(std::ffi::OsString::from_vec(bytes))
}

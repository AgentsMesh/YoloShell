const ROOT_FIRST_LOCK_SUCCESS: i32 = 0;
const ROOT_FIRST_TTY_NAME_SUCCESS: i32 = 0;
const ROOT_FIRST_STAGE_LOCK_READY: &[u8] = b"intermediate-lock-ready";
const ROOT_FIRST_STAGE_INTERMEDIATE_STARTED: &[u8] = b"intermediate-started";
const ROOT_FIRST_STAGE_LOCK_PATH_LOADED: &[u8] = b"intermediate-lock-path-loaded";
const ROOT_FIRST_STAGE_LOCK_OPENED: &[u8] = b"intermediate-lock-opened";
const ROOT_FIRST_STAGE_LOCK_ACQUIRED: &[u8] = b"intermediate-lock-acquired";
const ROOT_FIRST_STAGE_TTY_RESOLVED: &[u8] = b"intermediate-tty-resolved";
const ROOT_FIRST_STAGE_READY_PATH_LOADED: &[u8] = b"intermediate-ready-path-loaded";
const ROOT_FIRST_STAGE_READY_PUBLISHED: &[u8] = b"intermediate-ready-published";
const ROOT_FIRST_STAGE_RELEASE_PATH_LOADED: &[u8] = b"intermediate-release-path-loaded";
const ROOT_FIRST_STAGE_RELEASE_OBSERVED: &[u8] = b"intermediate-release-observed";
const ROOT_FIRST_STAGE_LEAF_SPAWNED: &[u8] = b"intermediate-leaf-spawned";
const ROOT_FIRST_STAGE_INTERMEDIATE_LEAVING: &[u8] = b"intermediate-leaving";
const ROOT_FIRST_STAGE_INTERMEDIATE_EXITED: &[u8] = b"intermediate-exited";
const ROOT_FIRST_STAGE_REOPEN_RELEASED: &[u8] = b"reopen-released";
const ROOT_FIRST_STAGE_LEAF_STARTED: &[u8] = b"leaf-started";
const ROOT_FIRST_STAGE_LEAF_SESSION_CREATED: &[u8] = b"leaf-session-created";
const ROOT_FIRST_STAGE_LEAF_LOCK_PATH_LOADED: &[u8] = b"leaf-lock-path-loaded";
const ROOT_FIRST_STAGE_LEAF_LOCK_OPENED: &[u8] = b"leaf-lock-opened";
const ROOT_FIRST_STAGE_LEAF_EXITED_EARLY: &str = "leaf-exited-before-launch";
const ROOT_FIRST_STAGE_STATUS_SEPARATOR: &str = ": ";
const ROOT_FIRST_STAGE_FIELD_SEPARATOR: &str = "; ";
const ROOT_FIRST_STAGE_PREVIOUS_LABEL: &str = "last leaf stage";
const ROOT_FIRST_STAGE_STDERR_LABEL: &str = "leaf stderr";
const ROOT_FIRST_LEAF_EARLY_EXIT_DETAIL: &str = "root-first leaf exited before launch marker";

enum IntermediateLockMode {
    Create,
    Existing,
}

#[test]
fn root_first_setsid_helper() {
    match std::env::var(ROOT_FIRST_HELPER_ROLE_ENV).as_deref() {
        Ok(ROOT_FIRST_INTERMEDIATE_ROLE) => run_root_first_intermediate(),
        Ok(ROOT_FIRST_LEAF_ROLE) => run_root_first_leaf(),
        _ => {}
    }
}

fn run_root_first_intermediate() {
    let helper_marker = std::env::var_os(ROOT_FIRST_HELPER_MARKER_ENV).unwrap();
    ignore_root_first_control_signals();
    publish_current_owner_cancellation_identity(OwnerCancellationHelperRole::Intermediate).unwrap();
    std::fs::write(&helper_marker, ROOT_FIRST_STAGE_INTERMEDIATE_STARTED).unwrap();
    let lock_path = std::env::var_os(ROOT_FIRST_INTERMEDIATE_LOCK_ENV).unwrap();
    std::fs::write(&helper_marker, ROOT_FIRST_STAGE_LOCK_PATH_LOADED).unwrap();
    let _lock = open_intermediate_lock(Path::new(&lock_path), IntermediateLockMode::Create);
    std::fs::write(&helper_marker, ROOT_FIRST_STAGE_LOCK_OPENED).unwrap();
    lock_exclusive(&_lock, ROOT_FIRST_INTERMEDIATE_LOCK_TIMEOUT_DETAIL);
    std::fs::write(&helper_marker, ROOT_FIRST_STAGE_LOCK_ACQUIRED).unwrap();
    let tty = root_first_terminal_path();
    std::fs::write(&helper_marker, ROOT_FIRST_STAGE_TTY_RESOLVED).unwrap();
    let ready = std::env::var_os(ROOT_FIRST_INTERMEDIATE_READY_ENV).unwrap();
    std::fs::write(&helper_marker, ROOT_FIRST_STAGE_READY_PATH_LOADED).unwrap();
    std::fs::write(ready, EMPTY_MARKER).unwrap();
    std::fs::write(&helper_marker, ROOT_FIRST_STAGE_READY_PUBLISHED).unwrap();
    let release = std::env::var_os(ROOT_FIRST_INTERMEDIATE_RELEASE_ENV).unwrap();
    std::fs::write(&helper_marker, ROOT_FIRST_STAGE_RELEASE_PATH_LOADED).unwrap();
    wait_for_helper_path(
        Path::new(&release),
        ROOT_FIRST_INTERMEDIATE_RELEASE_TIMEOUT_DETAIL,
    );
    std::fs::write(&helper_marker, ROOT_FIRST_STAGE_RELEASE_OBSERVED).unwrap();
    let launch_marker = std::env::var_os(ROOT_FIRST_LAUNCH_MARKER_ENV).unwrap();
    let leaf = crate::process::spawn_waitable_test_child(
        std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                BOOTSTRAP_TEST_EXACT_ARGUMENT,
                ROOT_FIRST_HELPER_TEST,
                BOOTSTRAP_TEST_NOCAPTURE_ARGUMENT,
                crate::constants::TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
            ])
            .env(ROOT_FIRST_HELPER_ROLE_ENV, ROOT_FIRST_LEAF_ROLE)
            .env(ROOT_FIRST_TTY_ENV, tty)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped()),
    );
    let mut leaf = RootFirstLeafChild::new(leaf);
    std::fs::write(&helper_marker, ROOT_FIRST_STAGE_LEAF_SPAWNED).unwrap();
    wait_for_leaf_launch(
        Path::new(&launch_marker),
        leaf.child_mut(),
        Path::new(&helper_marker),
    );
    std::fs::write(helper_marker, ROOT_FIRST_STAGE_INTERMEDIATE_LEAVING).unwrap();
    exit_root_first_helper();
}

fn run_root_first_leaf() {
    use std::os::unix::fs::OpenOptionsExt;
    let marker = std::env::var_os(ROOT_FIRST_HELPER_MARKER_ENV).unwrap();
    std::fs::write(&marker, ROOT_FIRST_STAGE_LEAF_STARTED).unwrap();
    publish_current_owner_cancellation_identity(OwnerCancellationHelperRole::Leaf).unwrap();
    unsafe {
        assert_ne!(libc::setsid(), SETSID_FAILURE);
    }
    std::fs::write(&marker, ROOT_FIRST_STAGE_LEAF_SESSION_CREATED).unwrap();
    ignore_root_first_control_signals();
    let lock_path = std::env::var_os(ROOT_FIRST_INTERMEDIATE_LOCK_ENV).unwrap();
    std::fs::write(&marker, ROOT_FIRST_STAGE_LEAF_LOCK_PATH_LOADED).unwrap();
    let _lock = open_intermediate_lock(Path::new(&lock_path), IntermediateLockMode::Existing);
    std::fs::write(&marker, ROOT_FIRST_STAGE_LEAF_LOCK_OPENED).unwrap();
    std::fs::write(&marker, ROOT_FIRST_STAGE_LOCK_READY).unwrap();
    let launch_marker = std::env::var_os(ROOT_FIRST_LAUNCH_MARKER_ENV).unwrap();
    std::fs::write(launch_marker, EMPTY_MARKER).unwrap();
    lock_exclusive(&_lock, ROOT_FIRST_LEAF_LOCK_TIMEOUT_DETAIL);
    std::fs::write(&marker, ROOT_FIRST_STAGE_INTERMEDIATE_EXITED).unwrap();
    let ready = std::env::var_os(ROOT_FIRST_REOPEN_READY_ENV).unwrap();
    std::fs::write(ready, EMPTY_MARKER).unwrap();
    let release = std::env::var_os(ROOT_FIRST_REOPEN_RELEASE_ENV).unwrap();
    wait_for_helper_path(
        Path::new(&release),
        ROOT_FIRST_REOPEN_RELEASE_TIMEOUT_DETAIL,
    );
    std::fs::write(&marker, ROOT_FIRST_STAGE_REOPEN_RELEASED).unwrap();
    let skip_reopen =
        std::env::var(ROOT_FIRST_SKIP_REOPEN_ENV).as_deref() == Ok(ROOT_FIRST_SKIP_REOPEN_VALUE);
    let _slave = (!skip_reopen).then(|| {
        let tty = std::env::var_os(ROOT_FIRST_TTY_ENV).unwrap();
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NOCTTY)
            .open(tty)
            .unwrap()
    });
    std::fs::write(&marker, std::process::id().to_string()).unwrap();
    wait_for_root_first_leaf_lifetime(Path::new(marker.as_os_str()));
}

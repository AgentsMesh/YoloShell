use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

static TERM_FORK_REQUESTED: AtomicBool = AtomicBool::new(false);

pub(super) fn run_initial() {
    let mode = std::env::var(FIXTURE_MODE_ENVIRONMENT).unwrap();
    let intermediate = std::env::var(FIXTURE_INTERMEDIATE_PID_ENVIRONMENT)
        .unwrap()
        .parse::<libc::pid_t>()
        .unwrap();
    if mode == FORK_AFTER_REFRESH_MODE {
        crate::process_identity_record::write(&required_path(
            FIXTURE_HISTORICAL_WITNESS_IDENTITY_ENVIRONMENT,
        ));
        wait_for_path(&required_path(
            FIXTURE_HISTORICAL_WITNESS_ACKNOWLEDGEMENT_ENVIRONMENT,
        ));
    }
    assert_ne!(unsafe { libc::setsid() }, SYSTEM_CALL_FAILURE);
    let deadline = Instant::now() + START_TIMEOUT;
    while unsafe { libc::getppid() } == intermediate {
        assert!(Instant::now() < deadline, "{FIXTURE_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
    match mode.as_str() {
        FORK_ON_TERM_MODE => {
            install_handler(term_fork_requested as *const () as libc::sighandler_t)
        }
        FORK_BEFORE_FORCE_MODE => install_handler(libc::SIG_IGN),
        FORK_AFTER_REFRESH_MODE => install_handler(libc::SIG_IGN),
        _ => {}
    }
    crate::process_identity_record::write(&required_path(FIXTURE_LEAF_IDENTITY_ENVIRONMENT));
    std::fs::write(required_path(FIXTURE_LEAF_READY_ENVIRONMENT), EMPTY_MARKER).unwrap();
    match mode.as_str() {
        FORK_ON_TERM_MODE => fork_after_term(),
        FORK_BEFORE_FORCE_MODE => fork_before_force(),
        FORK_AFTER_REFRESH_MODE => arm_refresh_holder(),
        _ => std::thread::sleep(FIXTURE_LIFETIME),
    }
}

pub(super) fn run_late() {
    let retiring_parent = (fixture_mode() == FORK_AFTER_REFRESH_MODE).then(|| {
        let pid = u32::try_from(unsafe { libc::getppid() }).unwrap();
        yolo_process_test_support::ProcessIdentity::capture(pid).unwrap()
    });
    install_handler(libc::SIG_DFL);
    assert_ne!(unsafe { libc::setsid() }, SYSTEM_CALL_FAILURE);
    crate::process_identity_record::write(&required_path(FIXTURE_LATE_LEAF_IDENTITY_ENVIRONMENT));
    if let Some(parent) = retiring_parent {
        let deadline = Instant::now() + START_TIMEOUT;
        while parent.matches() {
            assert!(Instant::now() < deadline, "{FIXTURE_TIMEOUT_DETAIL}");
            std::thread::sleep(POLL_INTERVAL);
        }
        acknowledge_refresh_handoff();
    }
    std::thread::sleep(FIXTURE_LIFETIME);
}

pub(super) fn run_refresh_holder() {
    let pid = u32::try_from(unsafe { libc::getppid() }).unwrap();
    let retiring_parent = yolo_process_test_support::ProcessIdentity::capture(pid).unwrap();
    assert_ne!(unsafe { libc::setsid() }, SYSTEM_CALL_FAILURE);
    crate::process_identity_record::write(&required_path(FIXTURE_REFRESH_HOLDER_READY_ENVIRONMENT));
    std::fs::write(
        required_path(FIXTURE_DISCOVERY_ARM_ACKNOWLEDGEMENT_ENVIRONMENT),
        EMPTY_MARKER,
    )
    .unwrap();
    wait_for_path(&required_path(FIXTURE_REFRESH_HANDOFF_REQUEST_ENVIRONMENT));
    std::fs::write(
        required_path(FIXTURE_COORDINATOR_RETIRE_REQUEST_ENVIRONMENT),
        EMPTY_MARKER,
    )
    .unwrap();
    wait_for_retirement(retiring_parent);
    spawn_late();
}

extern "C" fn term_fork_requested(_signal: libc::c_int) {
    TERM_FORK_REQUESTED.store(true, Ordering::Release);
}

fn fork_after_term() {
    let deadline = Instant::now() + FIXTURE_LIFETIME;
    while !TERM_FORK_REQUESTED.load(Ordering::Acquire) {
        assert!(Instant::now() < deadline, "{FIXTURE_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
    spawn_late();
    acknowledge_force_signal();
}

fn fork_before_force() {
    wait_for_path(&required_path(FIXTURE_FORCE_SIGNAL_REQUEST_ENVIRONMENT));
    spawn_late();
    acknowledge_force_signal();
    std::thread::sleep(FIXTURE_LIFETIME);
}

fn arm_refresh_holder() {
    wait_for_path(&required_path(FIXTURE_DISCOVERY_ARM_REQUEST_ENVIRONMENT));
    close_inherited_discovery_writer();
    let capability = std::env::var_os(FIXTURE_DISCOVERY_CAPABILITY_ENVIRONMENT).unwrap();
    let mut command = helper_command(REFRESH_HOLDER_ROLE);
    command
        .env(
            yolo_runtime::process::PROCESS_DISCOVERY_CAPABILITY_ENV,
            capability,
        )
        .env_remove(crate::fault_contract::DISCOVERY_WRITER_DESCRIPTOR_ENVIRONMENT);
    command.spawn().unwrap();
    wait_for_path(&required_path(FIXTURE_REFRESH_HOLDER_READY_ENVIRONMENT));
    wait_for_path(&required_path(
        FIXTURE_COORDINATOR_RETIRE_REQUEST_ENVIRONMENT,
    ));
}

fn acknowledge_force_signal() {
    std::fs::write(
        required_path(FIXTURE_FORCE_SIGNAL_ACKNOWLEDGEMENT_ENVIRONMENT),
        EMPTY_MARKER,
    )
    .unwrap();
}

fn acknowledge_refresh_handoff() {
    std::fs::write(
        required_path(FIXTURE_REFRESH_HANDOFF_ACKNOWLEDGEMENT_ENVIRONMENT),
        EMPTY_MARKER,
    )
    .unwrap();
}

fn wait_for_retirement(identity: yolo_process_test_support::ProcessIdentity) {
    let deadline = Instant::now() + START_TIMEOUT;
    while identity.matches() {
        assert!(Instant::now() < deadline, "{FIXTURE_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn spawn_late() {
    let mut command = helper_command(LATE_LEAF_ROLE);
    #[cfg(target_os = "macos")]
    command.env_remove(yolo_runtime::process::PROCESS_DESCRIPTOR_CAPABILITY_FD_ENV);
    command.env_remove(crate::fault_contract::DISCOVERY_WRITER_DESCRIPTOR_ENVIRONMENT);
    command.spawn().unwrap();
    wait_for_path(&required_path(FIXTURE_LATE_LEAF_IDENTITY_ENVIRONMENT));
}

#[cfg(target_os = "macos")]
fn close_inherited_discovery_writer() {
    use std::os::fd::{FromRawFd, OwnedFd, RawFd};

    let descriptor = std::env::var(crate::fault_contract::DISCOVERY_WRITER_DESCRIPTOR_ENVIRONMENT)
        .unwrap()
        .parse::<RawFd>()
        .unwrap();
    assert!(descriptor > libc::STDERR_FILENO);
    drop(unsafe { OwnedFd::from_raw_fd(descriptor) });
}

#[cfg(not(target_os = "macos"))]
fn close_inherited_discovery_writer() {}

fn install_handler(handler: libc::sighandler_t) {
    let previous = unsafe { libc::signal(libc::SIGTERM, handler) };
    assert_ne!(previous, libc::SIG_ERR);
}

fn fixture_mode() -> String {
    std::env::var(FIXTURE_MODE_ENVIRONMENT).unwrap()
}

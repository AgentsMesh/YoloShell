use crate::runtime_constants::{TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const HELPER_TEST: &str = "inventory::member_signal::integration_tests::signal_observer_helper";
const DIRECTORY_ENV: &str = "YOLO_MEMBER_SIGNAL_OBSERVER_DIRECTORY";
const READY_FILE: &str = "ready";
const RECEIVED_FILE: &str = "received-sigint";
const MARKER: &[u8] = b"observed";
const TIMEOUT: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(5);
const WAIT_FAILED: &str = "signal observer did not publish the expected marker";
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

pub(super) fn command(directory: &Path) -> std::process::Command {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            TEST_HARNESS_EXACT_ARGUMENT,
            HELPER_TEST,
            TEST_HARNESS_NOCAPTURE_ARGUMENT,
        ])
        .env(DIRECTORY_ENV, directory);
    command
}

pub(super) fn wait_ready(directory: &Path) {
    wait_for(&directory.join(READY_FILE));
}

pub(crate) fn wait_received(directory: &Path) {
    wait_for(&directory.join(RECEIVED_FILE));
}

pub(crate) fn received(directory: &Path) -> bool {
    directory.join(RECEIVED_FILE).exists()
}

fn wait_for(marker: &Path) {
    let deadline = Instant::now() + TIMEOUT;
    while !marker.exists() {
        assert!(Instant::now() < deadline, "{WAIT_FAILED}");
        std::thread::sleep(POLL);
    }
}

pub(crate) fn run_if_requested() {
    let Some(directory) = std::env::var_os(DIRECTORY_ENV) else {
        return;
    };
    let directory = PathBuf::from(directory);
    // Only the async-signal-safe atomic store runs in the handler.
    let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
    action.sa_sigaction = observe_interrupt as *const () as usize;
    assert_eq!(
        unsafe { libc::sigemptyset(&mut action.sa_mask) },
        i32::default()
    );
    assert_eq!(
        unsafe { libc::sigaction(libc::SIGINT, &action, std::ptr::null_mut()) },
        i32::default()
    );
    std::fs::write(directory.join(READY_FILE), MARKER).unwrap();
    loop {
        if INTERRUPTED.swap(false, Ordering::SeqCst) {
            std::fs::write(directory.join(RECEIVED_FILE), MARKER).unwrap();
        }
        std::thread::sleep(POLL);
    }
}

extern "C" fn observe_interrupt(signal: libc::c_int) {
    if signal == libc::SIGINT {
        INTERRUPTED.store(true, Ordering::SeqCst);
    }
}

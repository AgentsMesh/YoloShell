use super::*;
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

const EXIT_EXECUTABLE: &str = "/usr/bin/true";
const EXIT_WAIT_TIMEOUT: Duration = Duration::from_secs(2);
const POLL_INTERVAL: Duration = Duration::from_millis(5);

#[test]
fn unreaped_zombie_is_not_reported_alive() {
    let mut child = spawn_immediate_exit();
    let pid = child.id();
    wait_for_zombie(pid);

    assert!(!is_process_alive(pid));

    child.wait().unwrap();
}

fn spawn_immediate_exit() -> Child {
    crate::process::spawn_waitable_test_child(Command::new(EXIT_EXECUTABLE))
}

fn wait_for_zombie(pid: u32) {
    let deadline = Instant::now() + EXIT_WAIT_TIMEOUT;
    loop {
        if super::super::macos_kernel::metadata(pid).unwrap().is_none() {
            return;
        }
        assert!(Instant::now() < deadline, "process did not become a zombie");
        thread::sleep(POLL_INTERVAL);
    }
}

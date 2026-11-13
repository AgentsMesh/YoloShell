use super::super::constants::{
    CGROUP_EVENTS_FILE, CGROUP_FREEZE_FILE, CGROUP_PROCESSES_FILE, FREEZE_VALUE,
};
use super::super::{
    LinuxContainmentBroker, LinuxContainmentLease, LinuxContainmentRecord, descriptor,
};
use crate::process::{ProcessIdentity, process_start_time};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use uuid::Uuid;

pub const KERNEL_SETTLE_TIMEOUT: Duration = Duration::from_secs(3);
pub const KERNEL_POLL_INTERVAL: Duration = Duration::from_millis(10);
pub const SLEEP_EXECUTABLE: &str = "/bin/sleep";
pub const SLEEP_SECONDS_ARGUMENT: &str = "600";
pub const THAW_VALUE: &str = "0\n";

pub fn broker() -> LinuxContainmentBroker {
    LinuxContainmentBroker::discover()
        .expect("discover real cgroup v2 delegation")
        .expect("kernel gate requires a delegated cgroup root")
}

pub fn prepare() -> (Uuid, Uuid, LinuxContainmentLease) {
    let terminal_id = Uuid::new_v4();
    let runtime_epoch = Uuid::new_v4();
    let lease = broker()
        .prepare(terminal_id, runtime_epoch)
        .expect("create real session cgroup");
    (terminal_id, runtime_epoch, lease)
}

pub fn leaf_path(record: &LinuxContainmentRecord) -> PathBuf {
    record.delegated_root.join(&record.leaf_name)
}

pub fn leaf_identity(lease: &LinuxContainmentLease) -> super::super::CgroupDirectoryIdentity {
    descriptor::identity(&lease.leaf).expect("read session cgroup identity")
}

pub fn read_control(path: &Path, name: &str) -> String {
    fs::read_to_string(path.join(name)).expect("read real cgroup control")
}

pub fn write_freeze(path: &Path, value: &str) {
    fs::write(path.join(CGROUP_FREEZE_FILE), value).expect("write real cgroup freeze control");
}

pub fn assert_frozen(path: &Path) {
    wait_for_event(path, "frozen", true);
    assert_eq!(
        read_control(path, CGROUP_FREEZE_FILE).trim(),
        FREEZE_VALUE.trim()
    );
}

pub fn assert_unfrozen(path: &Path) {
    wait_for_event(path, "frozen", false);
}

pub fn assert_populated(path: &Path, expected: bool) {
    wait_for_event(path, "populated", expected);
}

pub fn freeze_members(
    lease: &mut LinuxContainmentLease,
) -> HashSet<crate::process::LiveProcessIdentity> {
    let deadline = Instant::now() + KERNEL_SETTLE_TIMEOUT;
    loop {
        if let Some(members) = lease
            .poll_frozen_members(true)
            .expect("freeze and observe real cgroup")
        {
            return members;
        }
        assert!(Instant::now() < deadline, "real cgroup did not freeze");
        std::thread::sleep(KERNEL_POLL_INTERVAL);
    }
}

pub fn wait_empty(path: &Path) {
    let deadline = Instant::now() + KERNEL_SETTLE_TIMEOUT;
    while !read_control(path, CGROUP_PROCESSES_FILE).trim().is_empty() {
        assert!(Instant::now() < deadline, "session cgroup stayed populated");
        std::thread::sleep(KERNEL_POLL_INTERVAL);
    }
    assert_populated(path, false);
}

pub fn move_member(path: &Path, identity: ProcessIdentity) {
    fs::write(
        path.join(CGROUP_PROCESSES_FILE),
        format!("{}\n", identity.pid),
    )
    .expect("move process into real cgroup");
}

pub fn wait_member_count(path: &Path, expected: usize) {
    let deadline = Instant::now() + KERNEL_SETTLE_TIMEOUT;
    loop {
        let count = read_control(path, CGROUP_PROCESSES_FILE).lines().count();
        if count == expected {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "cgroup membership did not converge"
        );
        std::thread::sleep(KERNEL_POLL_INTERVAL);
    }
}

fn wait_for_event(path: &Path, key: &str, expected: bool) {
    let expected = if expected { "1" } else { "0" };
    let deadline = Instant::now() + KERNEL_SETTLE_TIMEOUT;
    loop {
        let matched = read_control(path, CGROUP_EVENTS_FILE)
            .lines()
            .filter_map(|line| line.split_once(' '))
            .any(|(name, value)| name == key && value == expected);
        if matched {
            return;
        }
        assert!(Instant::now() < deadline, "cgroup event did not converge");
        std::thread::sleep(KERNEL_POLL_INTERVAL);
    }
}

pub struct SleepingChild {
    child: Child,
    identity: ProcessIdentity,
    finished: bool,
}

impl SleepingChild {
    pub fn spawn() -> Self {
        let child = crate::process::spawn_waitable_test_child(
            Command::new(SLEEP_EXECUTABLE)
                .arg(SLEEP_SECONDS_ARGUMENT)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null()),
        );
        let pid = child.id();
        let identity = ProcessIdentity {
            pid,
            start_time: process_start_time(pid).expect("capture cgroup member identity"),
        };
        Self {
            child,
            identity,
            finished: false,
        }
    }

    pub fn identity(&self) -> ProcessIdentity {
        self.identity
    }

    pub fn terminate(&mut self) {
        if !self.finished {
            let _ = self.child.kill();
            let _ = self.child.wait();
            self.finished = true;
        }
    }
}

impl Drop for SleepingChild {
    fn drop(&mut self) {
        self.terminate();
    }
}

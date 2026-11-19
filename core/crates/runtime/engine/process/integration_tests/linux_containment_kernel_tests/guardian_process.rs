use super::super::constants::{CGROUP_EVENTS_FILE, CGROUP_FREEZE_FILE, FREEZE_VALUE};
use super::guardian_monitor::{GuardianInput, next_input};
use super::support::{KERNEL_POLL_INTERVAL, KERNEL_SETTLE_TIMEOUT};
use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{FromRawFd, RawFd};
use std::time::Instant;

pub const ARM_COMMAND: u8 = b'a';
pub const RELEASE_COMMAND: u8 = b'r';
pub const ARMED_EVENT: u8 = b'A';
pub const RELEASED_EVENT: u8 = b'R';
pub const WORKER_LOST_EVENT: u8 = b'W';
pub const FAULT_EVENT: u8 = b'F';
const FROZEN_EVENT_LINE: &[u8] = b"frozen 1";
const EVENT_BUFFER_BYTES: usize = 256;
const SINGLE_MESSAGE_BYTE: usize = 1;
const CHILD_SUCCESS: libc::c_int = 0;
pub fn run_guardian(
    channel: RawFd,
    inherited_parent_channel: RawFd,
    master: RawFd,
    slave: RawFd,
    leaf: RawFd,
    unrelated: RawFd,
    worker_liveness: RawFd,
) -> ! {
    unsafe {
        libc::close(inherited_parent_channel);
        libc::close(slave);
        libc::close(unrelated);
    }
    let mut liveness = Some(worker_liveness);
    loop {
        match next_input(channel, liveness) {
            GuardianInput::WorkerLost => {
                respond(channel, freeze_exact(leaf), WORKER_LOST_EVENT);
                if let Some(descriptor) = liveness.take() {
                    unsafe { libc::close(descriptor) };
                }
            }
            GuardianInput::Command(ARM_COMMAND) => {
                respond(channel, freeze_exact(leaf), ARMED_EVENT)
            }
            GuardianInput::Command(RELEASE_COMMAND) => {
                if !freeze_exact(leaf) {
                    send(channel, FAULT_EVENT);
                    continue;
                }
                unsafe { libc::close(master) };
                send(channel, RELEASED_EVENT);
                exit_success();
            }
            GuardianInput::ControlLost => {
                if freeze_exact(leaf) {
                    unsafe { libc::close(master) };
                    exit_success();
                }
            }
            GuardianInput::Command(_) | GuardianInput::Fault => send(channel, FAULT_EVENT),
        }
    }
}

pub fn run_worker(master: RawFd, slave: RawFd, _liveness_writer: RawFd) -> ! {
    unsafe {
        libc::close(slave);
        libc::signal(libc::SIGHUP, libc::SIG_IGN);
        loop {
            libc::pause();
        }
    }
}

fn freeze_exact(leaf: RawFd) -> bool {
    if !write_freeze(leaf) {
        return false;
    }
    let deadline = Instant::now() + KERNEL_SETTLE_TIMEOUT;
    while Instant::now() < deadline {
        if read_frozen(leaf) {
            return true;
        }
        std::thread::sleep(KERNEL_POLL_INTERVAL);
    }
    false
}

fn write_freeze(leaf: RawFd) -> bool {
    let Some(mut file) = open_control(leaf, CGROUP_FREEZE_FILE, libc::O_WRONLY) else {
        return false;
    };
    file.write_all(FREEZE_VALUE.as_bytes()).is_ok()
}

fn read_frozen(leaf: RawFd) -> bool {
    let Some(mut file) = open_control(leaf, CGROUP_EVENTS_FILE, libc::O_RDONLY) else {
        return false;
    };
    let mut bytes = [u8::default(); EVENT_BUFFER_BYTES];
    let Ok(count) = file.read(&mut bytes) else {
        return false;
    };
    bytes[..count]
        .split(|byte| *byte == b'\n')
        .any(|line| line == FROZEN_EVENT_LINE)
}

fn open_control(leaf: RawFd, name: &str, flags: libc::c_int) -> Option<File> {
    let name = CString::new(name).ok()?;
    let descriptor = unsafe { libc::openat(leaf, name.as_ptr(), flags | libc::O_CLOEXEC) };
    (descriptor >= 0).then(|| unsafe { File::from_raw_fd(descriptor) })
}

fn respond(channel: RawFd, success: bool, event: u8) {
    send(channel, if success { event } else { FAULT_EVENT });
}

fn send(channel: RawFd, event: u8) {
    let count = unsafe {
        libc::send(
            channel,
            (&event as *const u8).cast(),
            SINGLE_MESSAGE_BYTE,
            libc::MSG_NOSIGNAL,
        )
    };
    if count != SINGLE_MESSAGE_BYTE as isize {
        exit_success();
    }
}

fn exit_success() -> ! {
    unsafe { libc::_exit(CHILD_SUCCESS) }
}

use super::test_constants::{
    ESCAPED_PID_DESCRIPTOR, SLEEP_EXECUTABLE, UNRELATED_PROCESS_SLEEP_SECONDS,
};
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::process::{Child, Command};
use yolo_process::{PROCESS_DISCOVERY_CAPABILITY_ENV, ProcessIdentity, ProcessIdentityObservation};

const FIXTURE_IO_EXIT: libc::c_int = 3;

pub(super) fn write_identity(identity: ProcessIdentity) {
    write_all(ESCAPED_PID_DESCRIPTOR, &identity.pid.to_ne_bytes());
    write_all(ESCAPED_PID_DESCRIPTOR, &identity.start_time.to_ne_bytes());
    assert_eq!(unsafe { libc::fsync(ESCAPED_PID_DESCRIPTOR) }, 0);
    unsafe { libc::close(ESCAPED_PID_DESCRIPTOR) };
}

pub(super) fn read_ready_pid(descriptor: libc::c_int) -> u32 {
    let mut bytes = [0; std::mem::size_of::<u32>()];
    read_all(descriptor, &mut bytes);
    u32::from_ne_bytes(bytes)
}

pub(super) fn read_identity(file: &File) -> ProcessIdentity {
    let mut pid = [0; std::mem::size_of::<u32>()];
    let mut start_time = [0; std::mem::size_of::<u64>()];
    assert_eq!(file.read_at(&mut pid, 0).unwrap(), pid.len());
    assert_eq!(
        file.read_at(&mut start_time, pid.len() as u64).unwrap(),
        start_time.len()
    );
    ProcessIdentity {
        pid: u32::from_ne_bytes(pid),
        start_time: u64::from_ne_bytes(start_time),
    }
}

pub(super) fn write_all(descriptor: libc::c_int, mut bytes: &[u8]) {
    while !bytes.is_empty() {
        let count = unsafe { libc::write(descriptor, bytes.as_ptr().cast(), bytes.len()) };
        if count <= 0 {
            unsafe { libc::_exit(FIXTURE_IO_EXIT) };
        }
        bytes = &bytes[count as usize..];
    }
}

fn read_all(descriptor: libc::c_int, mut bytes: &mut [u8]) {
    while !bytes.is_empty() {
        let count = unsafe { libc::read(descriptor, bytes.as_mut_ptr().cast(), bytes.len()) };
        assert!(count > 0);
        bytes = &mut bytes[count as usize..];
    }
}

pub(super) fn assert_gone(identity: ProcessIdentity) {
    assert_eq!(identity.observe(), ProcessIdentityObservation::Gone);
}

pub(super) struct UnrelatedProcess {
    child: Child,
    pub identity: ProcessIdentity,
}

impl UnrelatedProcess {
    pub(super) fn spawn() -> Self {
        let child = Command::new(SLEEP_EXECUTABLE)
            .arg(UNRELATED_PROCESS_SLEEP_SECONDS)
            .env_remove(PROCESS_DISCOVERY_CAPABILITY_ENV)
            .spawn()
            .unwrap();
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        Self { child, identity }
    }
}

impl Drop for UnrelatedProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

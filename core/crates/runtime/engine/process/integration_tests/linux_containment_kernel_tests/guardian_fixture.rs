use super::super::LinuxContainmentLease;
use super::devpts_fixture::{ForkWriter, PtyPair};
use super::guardian_child::{ChildProcess, wait_readable};
use super::guardian_process::{
    ARM_COMMAND, ARMED_EVENT, RELEASE_COMMAND, RELEASED_EVENT, WORKER_LOST_EVENT, run_guardian,
    run_worker,
};
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};

const INVALID_DESCRIPTOR: libc::c_int = -1;
const SYSTEM_CALL_SUCCESS: libc::c_int = 0;
const SINGLE_EVENT_BYTE: usize = 1;

pub struct ExternalGuardian {
    process: ChildProcess,
    channel: File,
}

impl ExternalGuardian {
    pub fn spawn(
        old: &PtyPair,
        lease: &LinuxContainmentLease,
        writer: &ForkWriter,
        worker: &mut MasterOwningWorker,
    ) -> Self {
        let mut channels = [INVALID_DESCRIPTOR; 2];
        let kind = libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC;
        assert_eq!(
            unsafe { libc::socketpair(libc::AF_UNIX, kind, 0, channels.as_mut_ptr()) },
            SYSTEM_CALL_SUCCESS
        );
        let liveness = worker.take_liveness();
        let pid = unsafe { libc::fork() };
        assert!(pid >= 0, "fork external PTY guardian");
        if pid == 0 {
            run_guardian(
                channels[1],
                channels[0],
                old.master.as_raw_fd(),
                old.slave_descriptor(),
                lease.leaf.as_raw_fd(),
                writer.signal_descriptor(),
                liveness.as_raw_fd(),
            );
        }
        unsafe { libc::close(channels[1]) };
        drop(liveness);
        Self {
            process: ChildProcess::new(pid),
            channel: unsafe { File::from_raw_fd(channels[0]) },
        }
    }

    pub fn arm_close_pending(&mut self) {
        self.exchange(ARM_COMMAND, ARMED_EVENT);
    }

    pub fn release_alias(&mut self) {
        self.exchange(RELEASE_COMMAND, RELEASED_EVENT);
        self.process.wait_for_exit();
    }

    pub fn await_worker_loss(&mut self) {
        self.read_event(WORKER_LOST_EVENT);
    }

    pub fn kill(&mut self) {
        self.process.kill();
    }

    fn exchange(&mut self, command: u8, expected: u8) {
        self.channel
            .write_all(&[command])
            .expect("send guardian command");
        self.read_event(expected);
    }

    fn read_event(&mut self, expected: u8) {
        wait_readable(self.channel.as_raw_fd());
        let mut event = [u8::default(); SINGLE_EVENT_BYTE];
        self.channel
            .read_exact(&mut event)
            .expect("read guardian event");
        assert_eq!(event[0], expected, "guardian failed closed");
    }
}

pub struct MasterOwningWorker {
    process: ChildProcess,
    liveness: Option<File>,
}

impl MasterOwningWorker {
    pub fn spawn(old: &PtyPair) -> Self {
        let mut liveness = [INVALID_DESCRIPTOR; 2];
        assert_eq!(
            unsafe { libc::pipe2(liveness.as_mut_ptr(), libc::O_CLOEXEC) },
            SYSTEM_CALL_SUCCESS
        );
        let pid = unsafe { libc::fork() };
        assert!(pid >= 0, "fork PTY-owning worker");
        if pid == 0 {
            unsafe { libc::close(liveness[0]) };
            run_worker(old.master.as_raw_fd(), old.slave_descriptor(), liveness[1]);
        }
        unsafe { libc::close(liveness[1]) };
        Self {
            process: ChildProcess::new(pid),
            liveness: Some(unsafe { File::from_raw_fd(liveness[0]) }),
        }
    }

    pub fn kill(&mut self) {
        self.process.kill();
    }

    fn take_liveness(&mut self) -> File {
        self.liveness
            .take()
            .expect("worker liveness endpoint must transfer once")
    }
}

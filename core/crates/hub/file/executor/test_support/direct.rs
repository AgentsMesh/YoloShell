use crate::{constants::*, fixture::deadline};
use std::{
    os::fd::OwnedFd,
    os::unix::net::UnixStream,
    path::Path,
    process::{Command, ExitStatus, Stdio},
};
use tokio::time::Instant;
use yolo_hub_file_helper_protocol::{HELPER_EXIT_EXPIRED, Request};
use yolo_hub_transport::Framed;
use yolo_process::ScopedWaitableChild;

pub struct DirectChild {
    pub child: Option<ScopedWaitableChild>,
    pub owner: Option<UnixStream>,
    _control: Framed<tokio::net::UnixStream>,
}

impl DirectChild {
    pub async fn spawn(program: &Path, request: &Request) -> Self {
        let (control, child_control) = UnixStream::pair().unwrap();
        let (owner, child_owner) = UnixStream::pair().unwrap();
        control.set_nonblocking(true).unwrap();
        let mut channel = Framed::new(tokio::net::UnixStream::from_std(control).unwrap());
        let mut command = Command::new(program);
        command
            .args(yolo_launch_contract::ProcessRole::FileHelper.arguments())
            .stdin(Stdio::from(OwnedFd::from(child_control)))
            .stdout(Stdio::null())
            .stderr(Stdio::from(OwnedFd::from(child_owner)));
        let child = ScopedWaitableChild::spawn_detached_before_deadline(
            &mut command,
            deadline().into_std(),
        )
        .unwrap();
        drop(command);
        channel.send(request, deadline()).await.unwrap();
        Self {
            child: Some(child),
            owner: Some(owner),
            _control: channel,
        }
    }
    pub async fn exited(&mut self, deadline: Instant) -> ExitStatus {
        tokio::time::timeout_at(deadline, async {
            loop {
                let child = self.child.as_mut().unwrap();
                if let Some(status) = child.try_wait().unwrap() {
                    let pid = child.process_id();
                    child.retire_reaped().unwrap();
                    self.child.take();
                    if status.code() == Some(HELPER_EXIT_EXPIRED) {
                        // The owner watchdog uses _exit, bypassing LLVM's normal final flush.
                        yolo_coverage_test_support::discard_aborted_process_profile(pid)
                            .expect(ABORTED_PROFILE_CLEANUP);
                    }
                    return status;
                }
                tokio::time::sleep(POLL_INTERVAL).await;
            }
        })
        .await
        .expect("file helper did not exit within its ownership deadline")
    }
    pub async fn terminate(&mut self) {
        let child = self.child.take().unwrap();
        let pid = child.process_id();
        let status = child
            .terminate_and_wait_status(deadline().into_std())
            .await
            .unwrap();
        yolo_coverage_test_support::discard_reaped_kill_profile(pid, status)
            .expect(ABORTED_PROFILE_CLEANUP);
    }
}

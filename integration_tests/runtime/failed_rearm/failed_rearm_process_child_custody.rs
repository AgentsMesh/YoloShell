use super::*;
use anyhow::{Context, Result, bail, ensure};
use std::io::{Read, Write};
use std::process::{Child, ExitStatus};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use yolo_runtime::process::ProcessIdentity;

pub(super) struct Completed {
    pub(super) status: ExitStatus,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
}

pub(super) fn spawn(command: &mut Command) -> Result<ChildGuard> {
    spawn_with_capture(command, capture_identity)
}

fn capture_identity(pid: u32) -> Result<ProcessIdentity> {
    let deadline = Instant::now() + IDENTITY_CAPTURE_TIMEOUT;
    loop {
        match ProcessIdentity::capture(pid) {
            Ok(identity) => return Ok(identity),
            Err(error) if Instant::now() >= deadline => return Err(error),
            Err(_) => std::thread::sleep(IDENTITY_CAPTURE_POLL_INTERVAL),
        }
    }
}

fn spawn_with_capture(
    command: &mut Command,
    capture: impl FnOnce(u32) -> Result<ProcessIdentity>,
) -> Result<ChildGuard> {
    let child = command.spawn()?;
    let mut custody = UncapturedChild::new(child);
    let identity = capture(custody.id()?)?;
    ChildGuard::bind(custody.take()?, identity)
}

struct UncapturedChild(Option<Child>);

impl UncapturedChild {
    fn new(child: Child) -> Self {
        Self(Some(child))
    }

    fn id(&self) -> Result<u32> {
        self.0.as_ref().map(Child::id).context(FIXTURE_CHILD_STATUS)
    }

    fn take(&mut self) -> Result<Child> {
        self.0.take().context(FIXTURE_CHILD_STATUS)
    }
}

impl Drop for UncapturedChild {
    fn drop(&mut self) {
        settle_child(&mut self.0, None);
    }
}

pub(super) struct ChildGuard {
    child: Option<Child>,
    identity: ProcessIdentity,
    stdout: Option<JoinHandle<std::io::Result<Vec<u8>>>>,
    stderr: Option<JoinHandle<std::io::Result<Vec<u8>>>>,
}

impl ChildGuard {
    fn bind(child: Child, identity: ProcessIdentity) -> Result<Self> {
        let mut guard = Self {
            child: Some(child),
            identity,
            stdout: None,
            stderr: None,
        };
        guard.start_drains()?;
        Ok(guard)
    }

    fn start_drains(&mut self) -> Result<()> {
        let child = self.child.as_mut().context(FIXTURE_CHILD_STATUS)?;
        self.stdout = Some(drain(child.stdout.take().context(FIXTURE_DRAIN_FAILURE)?));
        self.stderr = Some(drain(child.stderr.take().context(FIXTURE_DRAIN_FAILURE)?));
        Ok(())
    }

    pub(super) fn write_input(&mut self, input: &[u8]) -> Result<()> {
        self.child
            .as_mut()
            .context(FIXTURE_CHILD_STATUS)?
            .stdin
            .take()
            .context(FIXTURE_INPUT_CONTEXT)?
            .write_all(input)?;
        Ok(())
    }

    pub(super) fn wait(self) -> Result<Completed> {
        self.wait_for(CHILD_TIMEOUT)
    }

    fn wait_for(mut self, timeout: Duration) -> Result<Completed> {
        let deadline = Instant::now() + timeout;
        let status = loop {
            let child = self.child.as_mut().context(FIXTURE_CHILD_STATUS)?;
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if Instant::now() >= deadline {
                bail!(FIXTURE_CHILD_TIMEOUT);
            }
            std::thread::sleep(CHILD_POLL_INTERVAL);
        };
        self.child.take().context(FIXTURE_CHILD_STATUS)?;
        let stdout = join_drain(&mut self.stdout)?;
        let stderr = join_drain(&mut self.stderr)?;
        ensure!(!self.identity.matches(), FIXTURE_CHILD_REAP);
        Ok(Completed {
            status,
            stdout,
            stderr,
        })
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        settle_child(&mut self.child, Some(self.identity));
        let _ = join_drain(&mut self.stdout);
        let _ = join_drain(&mut self.stderr);
    }
}

fn drain(mut pipe: impl Read + Send + 'static) -> JoinHandle<std::io::Result<Vec<u8>>> {
    std::thread::spawn(move || {
        let mut output = Vec::new();
        pipe.read_to_end(&mut output)?;
        Ok(output)
    })
}

fn join_drain(drain: &mut Option<JoinHandle<std::io::Result<Vec<u8>>>>) -> Result<Vec<u8>> {
    drain
        .take()
        .context(FIXTURE_DRAIN_FAILURE)?
        .join()
        .map_err(|_| anyhow::anyhow!(FIXTURE_DRAIN_PANIC))?
        .context(FIXTURE_DRAIN_FAILURE)
}

fn settle_child(child: &mut Option<Child>, identity: Option<ProcessIdentity>) {
    let Some(mut child) = child.take() else {
        return;
    };
    if let Some(identity) = identity {
        let _ = yolo_process_test_support::send_signal(identity.pid, libc::SIGKILL);
    } else {
        let _ = child.kill();
    }
    let _ = child.wait();
}

#[cfg(test)]
pub(super) fn spawn_with_capture_for_test(
    command: &mut Command,
    capture: impl FnOnce(u32) -> Result<ProcessIdentity>,
) -> Result<ChildGuard> {
    spawn_with_capture(command, capture)
}

#[cfg(test)]
pub(super) fn wait_for_test(child: ChildGuard, timeout: Duration) -> Result<Completed> {
    child.wait_for(timeout)
}

#[cfg(test)]
pub(super) fn identity_for_test(child: &ChildGuard) -> ProcessIdentity {
    child.identity
}

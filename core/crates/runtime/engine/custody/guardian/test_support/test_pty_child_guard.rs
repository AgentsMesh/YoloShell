use crate::process::{OwnedPtyChild, ProcessIdentity, ProcessIdentityObservation};
use portable_pty::ExitStatus;
use std::sync::mpsc::{SyncSender, sync_channel};
use std::time::{Duration, Instant};

const CHILD_EXPECTATION: &str = "guarded PTY test child must retain direct custody";
const IDENTITY_CAPTURE_DETAIL: &str = "capture guarded PTY test child identity";
const PROCESS_ID_DETAIL: &str = "guarded PTY test child did not expose its process ID";
const REAPER_THREAD_DETAIL: &str = "reserve guarded PTY test child reaper";
const REAPER_THREAD_NAME: &str = "yolo-guardian-test-pty-reaper";
const SETTLEMENT_CHANNEL_CAPACITY: usize = 1;
const REAPER_POLL: Duration = crate::session_timing::GRACEFUL_TERMINATION_POLL_INTERVAL;

pub(in crate::custody_guardian) struct GuardedPtyChild {
    retained: Option<RetainedPtyChild>,
    identity: ProcessIdentity,
}

pub(super) struct RetainedPtyChild {
    child: Option<OwnedPtyChild>,
    identity: ProcessIdentity,
    settlement: Option<ReservedPtyChildSettlement>,
}

impl GuardedPtyChild {
    pub(in crate::custody_guardian) fn spawn(spawn: impl FnOnce() -> OwnedPtyChild) -> Self {
        let settlement = ReservedPtyChildSettlement::reserve();
        let spawn_guard = crate::process::waitable_child_spawn_guard().unwrap();
        let child = spawn();
        let identity = match capture_identity(&child) {
            Ok(identity) => identity,
            Err(error) => {
                drop(spawn_guard);
                settlement.terminate(child, None);
                panic!("{IDENTITY_CAPTURE_DETAIL}: {error:#}");
            }
        };
        drop(spawn_guard);
        Self {
            retained: Some(RetainedPtyChild {
                child: Some(child),
                identity,
                settlement: Some(settlement),
            }),
            identity,
        }
    }

    pub(in crate::custody_guardian) fn identity(&self) -> ProcessIdentity {
        self.identity
    }

    pub(in crate::custody_guardian) fn is_complete(&self) -> bool {
        self.retained.is_none()
    }

    pub(in crate::custody_guardian) fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
        let retained = self.retained.as_mut().expect(CHILD_EXPECTATION);
        let status = retained
            .child
            .as_mut()
            .expect(CHILD_EXPECTATION)
            .try_wait()?;
        if status.is_some() {
            retained.disarm();
            self.retained.take();
        }
        Ok(status)
    }

    pub(in crate::custody_guardian) fn wait_before(
        &mut self,
        deadline: Instant,
        poll: Duration,
    ) -> std::io::Result<Option<ExitStatus>> {
        while !self.is_complete() {
            if let Some(status) = self.try_wait()? {
                return Ok(Some(status));
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            std::thread::sleep(poll);
        }
        Ok(None)
    }

    pub(in crate::custody_guardian) fn request_cleanup(&self) -> anyhow::Result<()> {
        self.identity.enforce_exact_test_cleanup()
    }
}

impl Drop for GuardedPtyChild {
    fn drop(&mut self) {
        let identity = self.identity;
        super::release_after_cleanup(&mut self.retained, || identity.enforce_exact_test_cleanup());
    }
}

impl RetainedPtyChild {
    pub(super) fn for_test(child: OwnedPtyChild, identity: ProcessIdentity) -> Self {
        Self {
            child: Some(child),
            identity,
            settlement: Some(ReservedPtyChildSettlement::reserve()),
        }
    }

    fn disarm(&mut self) {
        self.child.take();
        self.settlement.take();
    }
}

impl Drop for RetainedPtyChild {
    fn drop(&mut self) {
        let Some(child) = self.child.take() else {
            return;
        };
        self.settlement
            .take()
            .expect(CHILD_EXPECTATION)
            .terminate(child, Some(self.identity));
    }
}

struct ReservedPtyChildSettlement {
    sender: Option<SyncSender<PtyChildSettlement>>,
}

struct PtyChildSettlement {
    child: OwnedPtyChild,
    identity: Option<ProcessIdentity>,
}

impl ReservedPtyChildSettlement {
    fn reserve() -> Self {
        let (sender, receiver) = sync_channel(SETTLEMENT_CHANNEL_CAPACITY);
        std::thread::Builder::new()
            .name(REAPER_THREAD_NAME.to_owned())
            .spawn(move || {
                if let Ok(settlement) = receiver.recv() {
                    reap(settlement);
                }
            })
            .expect(REAPER_THREAD_DETAIL);
        Self {
            sender: Some(sender),
        }
    }

    fn terminate(mut self, child: OwnedPtyChild, identity: Option<ProcessIdentity>) {
        let settlement = PtyChildSettlement { child, identity };
        let sender = self.sender.take().expect(CHILD_EXPECTATION);
        if let Err(error) = sender.send(settlement) {
            reap(error.0);
        }
    }
}

fn capture_identity(child: &OwnedPtyChild) -> anyhow::Result<ProcessIdentity> {
    let pid = child
        .process_id()
        .ok_or_else(|| anyhow::anyhow!(PROCESS_ID_DETAIL))?;
    ProcessIdentity::capture(pid)
}

fn reap(mut settlement: PtyChildSettlement) {
    loop {
        let observation =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| settlement.child.try_wait()));
        match observation {
            Ok(Ok(Some(_))) => return,
            Ok(Err(_)) | Err(_)
                if settlement.identity.is_some_and(|identity| {
                    identity.observe() == ProcessIdentityObservation::Gone
                }) =>
            {
                return;
            }
            Ok(Ok(None)) | Ok(Err(_)) | Err(_) => {}
        }
        if let Some(identity) = settlement.identity {
            let _ = std::panic::catch_unwind(|| identity.enforce_exact_test_cleanup());
        }
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| settlement.child.kill()));
        std::thread::sleep(REAPER_POLL);
    }
}

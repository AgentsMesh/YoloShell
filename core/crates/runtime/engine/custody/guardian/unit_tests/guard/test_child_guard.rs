#![cfg(test)]

#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../test_support/test_pty_child_guard.rs"]
mod pty;
pub(super) use pty::GuardedPtyChild;
#[path = "test_child_guard_containment_tests.rs"]
mod containment_tests;

use crate::process::{ProcessIdentity, ReservedDetachedChildSettlement};
use std::any::Any;
use std::process::{Child, ExitStatus};
use std::time::{Duration, Instant};

const CHILD_EXPECTATION: &str = "guarded test child must retain direct custody";
const IDENTITY_CAPTURE_DETAIL: &str = "capture guarded test child identity";

pub(super) struct GuardedChild {
    retained: Option<RetainedChild>,
    identity: ProcessIdentity,
}

struct RetainedChild {
    child: Option<Child>,
    settlement: Option<ReservedDetachedChildSettlement>,
}

impl GuardedChild {
    pub(super) fn spawn(spawn: impl FnOnce() -> Child) -> Self {
        let settlement = crate::process::reserve_detached_child_settlement().unwrap();
        let child = spawn();
        let identity = match ProcessIdentity::capture(child.id()) {
            Ok(identity) => identity,
            Err(error) => {
                settlement.terminate(child);
                panic!("{IDENTITY_CAPTURE_DETAIL}: {error:#}");
            }
        };
        Self {
            retained: Some(RetainedChild {
                child: Some(child),
                settlement: Some(settlement),
            }),
            identity,
        }
    }

    pub(super) fn identity(&self) -> ProcessIdentity {
        self.identity
    }

    pub(super) fn is_complete(&self) -> bool {
        self.retained.is_none()
    }

    pub(super) fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
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

    pub(super) fn wait_before(
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

    pub(super) fn request_cleanup(&self) -> anyhow::Result<()> {
        self.identity.enforce_exact_test_cleanup()
    }
}

impl Drop for GuardedChild {
    fn drop(&mut self) {
        let identity = self.identity;
        release_after_cleanup(&mut self.retained, || identity.enforce_exact_test_cleanup());
    }
}

impl RetainedChild {
    fn disarm(&mut self) {
        self.child.take();
        self.settlement.take();
    }
}

impl Drop for RetainedChild {
    fn drop(&mut self) {
        let Some(child) = self.child.take() else {
            return;
        };
        self.settlement
            .take()
            .expect(CHILD_EXPECTATION)
            .terminate(child);
    }
}

pub(super) fn release_after_cleanup<T>(
    retained: &mut Option<T>,
    cleanup: impl FnOnce() -> anyhow::Result<()>,
) {
    if retained.is_none() {
        return;
    }
    contain(cleanup);
    retained.take();
}

fn contain(operation: impl FnOnce() -> anyhow::Result<()>) {
    if let Err(payload) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation)) {
        dispose_payload(payload);
    }
}

fn dispose_payload(payload: Box<dyn Any + Send>) {
    if let Err(secondary) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(payload)))
    {
        std::mem::forget(secondary);
    }
}

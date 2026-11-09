use super::{HANDOFF_CHILD_CONTEXT, PendingBroker, PendingObservation, termination};
use anyhow::Result;
use std::process::Child;

impl PendingBroker {
    pub(crate) fn detached_for_test(child: Child) -> Self {
        let child = yolo_process::DetachedChildCustody::capture(child)
            .unwrap_or_else(yolo_process::DetachedChildCustodyError::into_custody);
        Self::detached(child, termination::reserve().expect(HANDOFF_CHILD_CONTEXT))
    }

    pub(crate) fn unmanaged() -> Self {
        Self {
            detached: None,
            supervised: None,
            observer: None,
            compensation: None,
        }
    }

    pub(crate) fn tracked<F>(compensation: F) -> Self
    where
        F: FnOnce() -> Result<()> + Send + 'static,
    {
        Self {
            detached: None,
            supervised: None,
            observer: None,
            compensation: Some(Box::new(compensation)),
        }
    }

    pub(crate) fn tracked_pending<O, C>(observer: O, compensation: C) -> Self
    where
        O: FnMut() -> Result<PendingObservation> + Send + 'static,
        C: FnOnce() -> Result<()> + Send + 'static,
    {
        Self {
            detached: None,
            supervised: None,
            observer: Some(Box::new(observer)),
            compensation: Some(Box::new(compensation)),
        }
    }

    pub(super) fn disarm_test_custody(&mut self) {
        self.observer.take();
        self.compensation.take();
    }
}

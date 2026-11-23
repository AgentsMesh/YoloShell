use crate::pending::{PendingObservation, PendingSpawn, PendingSpawnHandle};
use anyhow::Result;
use yolo_broker_child_custody::{PendingBroker, reserve_termination_custody};
use yolo_remote_broker::BrokerStatus;
#[path = "pending_task.rs"]
mod task;
pub(crate) use task::{TrackedBrokerTask, TrackedTaskSettlement};

pub(crate) struct FakePending {
    observer: Option<Box<dyn FnMut() -> Result<PendingObservation> + Send>>,
    compensation: Option<Box<dyn FnOnce() -> Result<()> + Send>>,
}

impl FakePending {
    pub(crate) fn unmanaged() -> PendingSpawnHandle {
        Box::new(Self {
            observer: None,
            compensation: None,
        })
    }
    pub(crate) fn tracked(
        compensation: impl FnOnce() -> Result<()> + Send + 'static,
    ) -> PendingSpawnHandle {
        Box::new(Self {
            observer: None,
            compensation: Some(Box::new(compensation)),
        })
    }
    pub(crate) fn tracked_pending(
        observer: impl FnMut() -> Result<PendingObservation> + Send + 'static,
        compensation: impl FnOnce() -> Result<()> + Send + 'static,
    ) -> PendingSpawnHandle {
        Box::new(Self {
            observer: Some(Box::new(observer)),
            compensation: Some(Box::new(compensation)),
        })
    }
    pub(crate) fn detached_for_test(child: std::process::Child) -> PendingSpawnHandle {
        let child = yolo_process::DetachedChildCustody::capture(child)
            .unwrap_or_else(yolo_process::DetachedChildCustodyError::into_custody);
        Box::new(PendingBroker::detached(
            child,
            reserve_termination_custody().unwrap(),
        ))
    }
}

impl PendingSpawn for FakePending {
    fn observe(&mut self) -> Result<PendingObservation> {
        self.observer
            .as_mut()
            .map_or(Ok(PendingObservation::Pending), |observe| observe())
    }
    fn matches_status(&self, _: &BrokerStatus) -> bool {
        true
    }
    fn adopt(mut self: Box<Self>) -> Result<()> {
        self.observer.take();
        self.compensation.take();
        Ok(())
    }
    fn compensate(mut self: Box<Self>) -> Result<()> {
        self.observer.take();
        self.compensation
            .take()
            .map_or(Ok(()), |compensate| compensate())
    }
}

impl Drop for FakePending {
    fn drop(&mut self) {
        if let Some(compensation) = self.compensation.take() {
            let _ = compensation();
        }
    }
}

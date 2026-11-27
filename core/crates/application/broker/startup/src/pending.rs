use anyhow::Result;
use yolo_broker_child_custody::PendingBroker;
pub(crate) use yolo_broker_child_custody::PendingObservation;
use yolo_remote_broker::BrokerStatus;

pub(crate) type PendingSpawnHandle = Box<dyn PendingSpawn>;

/// The coordinator can commit or compensate custody, never manufacture it.
pub(crate) trait PendingSpawn: Send {
    fn observe(&mut self) -> Result<PendingObservation>;
    fn matches_status(&self, status: &BrokerStatus) -> bool;
    fn adopt(self: Box<Self>) -> Result<()>;
    fn compensate(self: Box<Self>) -> Result<()>;
}

impl PendingSpawn for PendingBroker {
    fn observe(&mut self) -> Result<PendingObservation> {
        PendingBroker::observe(self)
    }
    fn matches_status(&self, status: &BrokerStatus) -> bool {
        PendingBroker::matches_status(self, status)
    }
    fn adopt(self: Box<Self>) -> Result<()> {
        PendingBroker::adopt(*self)
    }
    fn compensate(self: Box<Self>) -> Result<()> {
        PendingBroker::compensate(*self)
    }
}

use super::{PendingBroker, SupervisedBroker, TerminationReservation};
use std::time::Instant;
use yolo_broker_supervision_state::{SupervisionPaths, SupervisionRecord};
use yolo_process::DetachedChildCustody;

impl PendingBroker {
    pub fn from_supervision(
        child: DetachedChildCustody,
        termination: TerminationReservation,
        record: SupervisionRecord,
        paths: SupervisionPaths,
        deadline: Instant,
    ) -> Self {
        Self::supervised(SupervisedBroker::new(
            child,
            termination,
            record,
            paths,
            deadline,
        ))
    }
}

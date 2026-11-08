use super::TerminationReservation;
use yolo_process::DetachedChildCustody;

pub enum PendingObservation {
    Pending,
    Exited(String),
}

pub(super) struct DetachedBroker {
    pub(super) child: DetachedChildCustody,
    pub(super) termination: TerminationReservation,
}

use uuid::Uuid;
use yolo_hub_identity::RuntimeTarget;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Intent {
    Observe,
    Write,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Presentation {
    ReadOnly,
    Acquiring,
    Writing,
    Recovering,
    Occupied,
    Denied,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Failure {
    Contended,
    Transient,
    Denied,
}

/// Correlates one attempt across target replacement and control reconnection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Attempt {
    pub target: RuntimeTarget,
    pub request_id: Uuid,
    pub connection: Uuid,
}

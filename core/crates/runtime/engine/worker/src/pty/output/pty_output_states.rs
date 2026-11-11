#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OutputQueueState {
    Pending,
    Empty,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OutputProgressState {
    Pending,
    Quiescent,
    Complete,
    Abandoned,
}

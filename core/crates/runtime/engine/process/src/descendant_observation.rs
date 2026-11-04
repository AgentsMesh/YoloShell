#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DescendantObservation {
    Settled,
    CapabilityScanPending,
    Indeterminate,
}

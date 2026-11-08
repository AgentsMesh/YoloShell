#[derive(Debug, Eq, PartialEq)]
pub(crate) enum FreezeClosureCandidate {
    Exact(Vec<LiveProcessIdentity>),
    IncrementalExact(Vec<LiveProcessIdentity>),
    Ordinary(Vec<LiveProcessIdentity>),
    KernelEof(Vec<LiveProcessIdentity>),
}

enum StoppedCandidateScope {
    ExactPositiveOnly,
    AllAuthorities,
}

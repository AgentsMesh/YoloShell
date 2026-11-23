pub enum FrozenProducersGoneCommit {
    Ready,
    FreshEofRequired,
    ReopenSealed(crate::process::CommittedPtyReopenSealAuthority),
    Refreeze,
}

pub enum KernelEofReopenSealCommit {
    FreshEofRequired,
    Reusable(crate::process::CommittedPtyReopenSealAuthority),
}

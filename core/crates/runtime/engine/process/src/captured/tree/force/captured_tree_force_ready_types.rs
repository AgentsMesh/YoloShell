pub struct FreezePendingReady(Option<freeze::FreezeClosureCandidate>);

pub struct FrozenFreezeReady {
    closure: Vec<LiveProcessIdentity>,
    version: ProcessClosureVersion,
    kill_basis: FrozenKillBasis,
}

pub struct KernelEofFreezeReady {
    closure: Vec<LiveProcessIdentity>,
    version: ProcessClosureVersion,
    authority: Option<crate::process::kernel_eof_authority::KernelEofCommitBinding>,
}

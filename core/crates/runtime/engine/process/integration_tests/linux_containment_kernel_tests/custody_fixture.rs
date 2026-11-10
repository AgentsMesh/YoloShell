use super::super::LinuxContainmentLease;
use super::devpts_fixture::PtyPair;
use super::support::{SleepingChild, prepare};

const CUSTODY_PTY_CAPABILITY: &str = "kernel-custody-capability";

pub(super) fn prepare_bound_lease() -> (LinuxContainmentLease, SleepingChild, PtyPair) {
    let (_, _, mut lease) = prepare();
    let child = SleepingChild::spawn();
    lease
        .migrate_bootstrap(child.identity())
        .expect("bind foreign cgroup custody");
    let pty = bind_pty(&mut lease);
    (lease, child, pty)
}

pub(super) fn bind_pty(lease: &mut LinuxContainmentLease) -> PtyPair {
    let pty = PtyPair::open();
    let proof = crate::process::PtySessionProof::capture(
        pty.path.clone(),
        CUSTODY_PTY_CAPABILITY.to_owned(),
    )
    .expect("capture real PTY proof");
    lease.bind_pty_proof(&proof).expect("bind real PTY proof");
    pty
}

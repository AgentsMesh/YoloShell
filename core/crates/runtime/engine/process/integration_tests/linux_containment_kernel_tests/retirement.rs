use super::super::constants::{CGROUP_EVENTS_FILE, CGROUP_PROCESSES_FILE};
use super::support::{
    KERNEL_POLL_INTERVAL, KERNEL_SETTLE_TIMEOUT, SleepingChild, leaf_path, prepare, read_control,
};
use crate::process::{
    CapturedProcessTree, FreezePoll, KillCommitPoll, KillPoll, PtySessionProof,
    ReportedProducersGoneRetirement, RootGoneWithoutWait,
};
use std::time::Instant;

#[test]
fn removed_empty_cgroup_still_requires_exact_master_alias_release() {
    let (_, _, mut lease) = prepare();
    let path = leaf_path(lease.record());
    let pty = portable_pty::native_pty_system()
        .openpty(portable_pty::PtySize::default())
        .unwrap();
    let (mut proof, _) =
        PtySessionProof::capture_fresh_bootstrap_bound(pty.master.as_ref()).unwrap();
    proof
        .bind_runtime(lease.record().terminal_id, lease.record().runtime_epoch)
        .unwrap();
    lease.bind_pty_proof(&proof).unwrap();
    let mut child = SleepingChild::spawn();
    let root = child.identity();
    lease.migrate_bootstrap(root).unwrap();
    let _master_release = proof.bind_master_alias_release(root).unwrap();
    let mut tree = CapturedProcessTree::capture_pty(root, proof).unwrap();
    tree.install_linux_containment(lease).unwrap();
    child.terminate();
    let gone = RootGoneWithoutWait::observe_exact_generation(root, &tree.live_root())
        .unwrap()
        .unwrap();
    tree.mark_root_gone_without_wait(gone).unwrap();

    let deadline = Instant::now() + KERNEL_SETTLE_TIMEOUT;
    let mut freezing = tree.begin_force();
    let frozen = loop {
        match freezing.poll().unwrap() {
            FreezePoll::Pending(next) => freezing = next,
            FreezePoll::Frozen(frozen) => break frozen,
        }
        assert!(
            Instant::now() < deadline,
            "freeze did not converge: confirmation={}, events={:?}, members={:?}",
            freezing.confirmation_for_test(),
            read_control(&path, CGROUP_EVENTS_FILE),
            read_control(&path, CGROUP_PROCESSES_FILE),
        );
        std::thread::sleep(KERNEL_POLL_INTERVAL);
    };
    let version = frozen.closure_version();
    let KillCommitPoll::Pending(mut pending) = frozen.kill_after_output_fence(version).unwrap()
    else {
        panic!("an empty frozen cgroup cannot acquire another producer");
    };
    let gone = loop {
        match pending.poll().unwrap() {
            KillPoll::Pending(next) => pending = next,
            KillPoll::Gone(gone) => break gone,
            KillPoll::Refreeze(_) => panic!("empty cgroup unexpectedly required refreezing"),
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(KERNEL_POLL_INTERVAL);
    };
    assert!(!path.exists());
    let (mut reported, ()) = gone.report(|_| Ok(())).unwrap();
    let ready = reported.prepare_retirement().unwrap();
    let ReportedProducersGoneRetirement::MasterRelease(mut pending) =
        reported.into_retirement(ready).unwrap()
    else {
        panic!("cgroup removal cannot retire retained PTY master aliases");
    };
    assert!(
        pending
            .prepare_retirement_after_master_release(None)
            .is_err()
    );
    assert!(!pending.session_is_retired());
}

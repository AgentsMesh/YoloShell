use super::*;

const WORKER_KILL_CONTEXT: &str = "SIGKILL simulated power-loss worker identity";
const WORKER_SIGNAL_REJECTED_DETAIL: &str = "simulated power-loss worker identity changed";

pub(super) fn signal(records: &[BoundRuntimeRecord]) {
    let mut workers = records
        .iter()
        .map(|record| cleanup_tree::runtime_worker_identity(record.record()))
        .collect::<Vec<_>>();
    cleanup_tree::sort_identities(&mut workers);
    for worker in &workers {
        assert!(
            worker.signal(libc::SIGKILL).expect(WORKER_KILL_CONTEXT),
            "{WORKER_SIGNAL_REJECTED_DETAIL}: {worker:?}"
        );
    }
}

pub(super) async fn wait_for_exit(records: &[BoundRuntimeRecord]) {
    for record in records {
        wait_for_identity_gone(ProcessIdentity {
            pid: record.record().worker_pid,
            start_time: record.record().worker_start_time,
        })
        .await;
        discard_incomplete_coverage_profile(record.record().worker_pid);
    }
}

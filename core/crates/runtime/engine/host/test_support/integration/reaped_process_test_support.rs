fn reaped_test_process() -> crate::process::ProcessIdentity {
    let identity = crate::host::reaped_host_test_process_identity();
    assert_eq!(
        identity.observe(),
        crate::process::ProcessIdentityObservation::Gone
    );
    identity
}

fn install_reaped_runtime_processes(runtime: &mut RuntimeRecord) {
    let worker = reaped_test_process();
    let child = reaped_test_process();
    runtime.worker_pid = worker.pid;
    runtime.worker_start_time = worker.start_time;
    runtime.child_pid = child.pid;
    runtime.child_start_time = child.start_time;
}

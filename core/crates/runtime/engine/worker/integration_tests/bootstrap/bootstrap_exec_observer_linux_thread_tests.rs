use super::*;
#[path = "../../test_support/integration/bootstrap_exec/bootstrap_exec_observer_thread.rs"]
mod fixture;

#[test]
fn non_leader_exec_is_observed_after_the_kernel_changes_its_thread_id() {
    let mut child = fixture::Child::spawn();
    let target = child.target();
    assert_ne!(target.process.pid, target.thread_id);
    let observer = ExecObserver::register(target).unwrap();
    child.release();
    observer.wait_until(fixture::deadline()).unwrap();
    assert!(child.wait().success());
}

#[test]
fn non_leader_exec_helper() {
    fixture::run_helper();
}

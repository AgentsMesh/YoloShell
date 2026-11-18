use tokio::time::Instant;
use yolo_process::{ProcessIdentityObservation, ScopedWaitableChild};

pub async fn terminate_owned_child(child: ScopedWaitableChild, deadline: Instant) {
    let identity = child.captured_identity().unwrap();
    let status = child
        .terminate_and_wait_status(deadline.into_std())
        .await
        .unwrap();
    assert_eq!(identity.observe(), ProcessIdentityObservation::Gone);
    // A natural completion keeps its coverage. Only a waited SIGKILL can have
    // interrupted this deliberately retired helper's final LLVM profile flush.
    yolo_coverage_test_support::discard_reaped_kill_profile(identity.pid, status).unwrap();
}

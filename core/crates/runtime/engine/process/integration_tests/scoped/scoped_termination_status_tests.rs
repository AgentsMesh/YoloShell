use super::*;
use std::os::unix::process::ExitStatusExt;

#[tokio::test]
async fn termination_status_reports_the_reaped_signal() {
    let child = spawn_sleep();
    let pid = child.process_id();
    let status = child
        .terminate_and_wait_status(Instant::now() + OPERATION_TIMEOUT)
        .await
        .unwrap();
    assert_eq!(status.signal(), Some(libc::SIGKILL));
    assert_reaped(pid);
}

#[tokio::test]
async fn termination_preserves_an_already_observed_natural_exit() {
    for script in [CLEAN_EXIT_SCRIPT, FAILED_EXIT_SCRIPT] {
        let mut child = spawn_shell(script);
        let pid = child.process_id();
        let limit = Instant::now() + OPERATION_TIMEOUT;
        let expected = tokio::time::timeout_at(limit.into(), async {
            loop {
                if let Some(status) = child.try_wait().unwrap() {
                    break status;
                }
                tokio::time::sleep(REAP_POLL_INTERVAL).await;
            }
        })
        .await
        .unwrap();
        let actual = child.terminate_and_wait_status(limit).await.unwrap();
        assert_eq!(actual, expected);
        assert_eq!(actual.signal(), None);
        assert_reaped(pid);
    }
}

#[tokio::test]
async fn expired_termination_status_request_still_retires_its_owned_child() {
    let child = spawn_sleep();
    let pid = child.process_id();
    let error = child
        .terminate_and_wait_status(Instant::now())
        .await
        .unwrap_err();
    assert!(format!("{error:#}").contains(WAIT_DEADLINE_EXPIRED));
    wait_until_reaped(pid).await;
}

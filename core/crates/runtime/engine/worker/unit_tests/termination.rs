use super::*;

const STUBBORN_SHELL: &str = "/bin/sh";
const IGNORE_TERM_SCRIPT: &str = "trap '' TERM; while :; do sleep 1; done";
const SHELL_COMMAND_ARGUMENT: &str = "-c";
const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION_ARGUMENT: &str = "30";
const CHILD_READY_DELAY: Duration = Duration::from_millis(100);

#[tokio::test]
async fn graceful_termination_escalates_for_a_process_ignoring_term() {
    let child = process::spawn_detached(
        Path::new(STUBBORN_SHELL),
        &[SHELL_COMMAND_ARGUMENT.into(), IGNORE_TERM_SCRIPT.into()],
        process::DetachedStandardError::Null,
    )
    .unwrap();
    let pid = child.id();
    tokio::time::sleep(CHILD_READY_DELAY).await;
    let scope =
        process::PtyProcessScope::capture(process::ProcessIdentity::capture(pid).unwrap()).unwrap();
    let waiter = std::thread::spawn(move || wait_for_test_child(child));
    terminate_process_scope(scope, false).await.unwrap();
    let status = waiter.join().unwrap();
    assert!(!status.success());
    assert!(!process::is_process_alive(pid));
}

#[tokio::test]
async fn force_termination_does_not_wait_for_the_grace_period() {
    let child = process::spawn_detached(
        Path::new(SLEEP_EXECUTABLE),
        &[SLEEP_DURATION_ARGUMENT.into()],
        process::DetachedStandardError::Null,
    )
    .unwrap();
    let pid = child.id();
    let scope =
        process::PtyProcessScope::capture(process::ProcessIdentity::capture(pid).unwrap()).unwrap();
    let waiter = std::thread::spawn(move || wait_for_test_child(child));
    let started = Instant::now();
    terminate_process_scope(scope, true).await.unwrap();
    let status = waiter.join().unwrap();
    assert!(!status.success());
    assert!(started.elapsed() < GRACEFUL_TERMINATION_TIMEOUT);
}

fn wait_for_test_child(mut child: std::process::Child) -> std::process::ExitStatus {
    child.wait().unwrap()
}

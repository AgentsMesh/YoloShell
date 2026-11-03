use super::*;

const LONG_SLEEP_SECONDS: &str = "60";

#[tokio::test]
async fn child_guard_kills_and_waits_when_a_test_scope_aborts() {
    let identity = {
        let child = Command::new("/bin/sleep")
            .arg(LONG_SLEEP_SECONDS)
            .spawn()
            .unwrap();
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        let _guard = ChildGuard::new(child);
        identity
    };

    wait_for_identity_gone(identity).await;
}

use super::{constants::*, fixture::Fixture, *};
use std::{
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use yolo_process::ScopedWaitableChild;

const REVOKE_ON_CHECK: usize = 5;
const REVOKED_PROFILE: &str = "profile changed after initial grant";

#[tokio::test]
async fn owner_revocation_after_initial_grant_prevents_final_handoff() {
    let fixture = Fixture::new();
    let checks = Arc::new(AtomicUsize::default());
    let observed = checks.clone();
    let (mut issuer, input) = issue(
        fixture.owner.clone(),
        fixture.scope.clone(),
        None,
        move || {
            let previous = observed.fetch_add(EXPECTED_STAGES, Ordering::SeqCst);
            anyhow::ensure!(previous < REVOKE_ON_CHECK, REVOKED_PROFILE);
            Ok(())
        },
    )
    .unwrap();
    let deadline = fixture.scope.deadline().unwrap();
    let mut command = Command::new(std::env::var(CHILD_BINARY).unwrap());
    command
        .arg(COMPLETE)
        .stdin(Stdio::from(input))
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let child =
        ScopedWaitableChild::spawn_before_deadline(&mut command, deadline.into_std()).unwrap();
    issuer.bind_child(&child).unwrap();
    let error = issuer.serve().await.unwrap_err();
    assert!(error.to_string().contains(REVOKED_PROFILE));
    assert!(checks.load(Ordering::SeqCst) > REVOKE_ON_CHECK);
    retire::terminate_owned_child(child, deadline).await;
}

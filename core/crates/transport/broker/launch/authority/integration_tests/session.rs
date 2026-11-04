use super::{constants::*, fixture::Fixture, *};
use std::{
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use yolo_process::ScopedWaitableChild;

fn command(input: std::os::fd::OwnedFd, mode: &str) -> Command {
    let mut command = Command::new(std::env::var(CHILD_BINARY).unwrap());
    command
        .arg(mode)
        .stdin(Stdio::from(input))
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    command
}

#[tokio::test]
async fn captured_child_receives_live_authority_and_only_finished_reports_adopted_ready() {
    let fixture = Fixture::new();
    let checked = Arc::new(AtomicUsize::default());
    let observed = checked.clone();
    let (mut issuer, input) = issue(
        fixture.owner.clone(),
        fixture.scope.clone(),
        None,
        move || {
            observed.fetch_add(EXPECTED_STAGES, Ordering::SeqCst);
            Ok(())
        },
    )
    .unwrap();
    let deadline = fixture.scope.deadline().unwrap();
    let child = ScopedWaitableChild::spawn_before_deadline(
        &mut command(input, COMPLETE),
        deadline.into_std(),
    )
    .unwrap();
    issuer.bind_child(&child).unwrap();
    assert!(issuer.bind_child(&child).is_err());
    let status = issuer.serve().await.unwrap();
    assert!(status.adopted);
    assert_eq!(status.host_id, fixture.scope.profile.profile_id());
    assert_eq!(status.config_revision, fixture.scope.config_revision);
    assert!(checked.load(Ordering::SeqCst) > EXPECTED_STAGES);
    child
        .wait_for_clean_exit(deadline.into_std())
        .await
        .unwrap();
}

#[tokio::test]
async fn commit_ack_and_a_second_commit_cannot_be_reported_as_completed_adoption() {
    for mode in [ABANDON, REPLAY] {
        let fixture = Fixture::new();
        let (mut issuer, input) =
            issue(
                fixture.owner.clone(),
                fixture.scope.clone(),
                None,
                || Ok(()),
            )
            .unwrap();
        let deadline = fixture.scope.deadline().unwrap();
        let child = ScopedWaitableChild::spawn_before_deadline(
            &mut command(input, mode),
            deadline.into_std(),
        )
        .unwrap();
        issuer.bind_child(&child).unwrap();
        assert!(issuer.serve().await.is_err());
        child
            .wait_for_clean_exit(deadline.into_std())
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn bound_child_identity_cannot_be_replaced_by_another_live_child() {
    let fixture = Fixture::new();
    let (mut issuer, input) = issue(
        fixture.owner.clone(),
        fixture.scope.clone(),
        None,
        || Ok(()),
    )
    .unwrap();
    let deadline = fixture.scope.deadline().unwrap();
    let child = ScopedWaitableChild::spawn_before_deadline(
        &mut command(input, COMPLETE),
        deadline.into_std(),
    )
    .unwrap();
    let mut sleeper = Command::new(SLEEP_EXECUTABLE);
    sleeper
        .arg(SLEEP_SECONDS)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let other =
        ScopedWaitableChild::spawn_before_deadline(&mut sleeper, deadline.into_std()).unwrap();
    issuer.bind_child(&other).unwrap();
    assert!(issuer.serve().await.is_err());
    retire::terminate_owned_child(child, deadline).await;
    retire::terminate_owned_child(other, deadline).await;
}

#[tokio::test]
async fn credentials_travel_only_in_the_inherited_private_channel() {
    let fixture = Fixture::new();
    let credentials = zeroize::Zeroizing::new(
        yolo_capnp_codec::encode(&serde_json::json!({
            (VERSION_FIELD):yolo_remote_broker::IDENTITY_PASSPHRASE_WIRE_VERSION,
            (BINDING_NONCE_FIELD):CREDENTIAL_NONCE,
            (HOST_ID_FIELD):PROFILE,
            (IDENTITY_CREDENTIAL_REVISION_FIELD):null,
            (IDENTITY_PATHS_SHA256_FIELD):REVISION,
            (PASSPHRASES_FIELD):[EXPECTED_SECRET],
        }))
        .unwrap(),
    );
    let (mut issuer, input) = issue(
        fixture.owner.clone(),
        fixture.scope.clone(),
        Some(credentials),
        || Ok(()),
    )
    .unwrap();
    let deadline = fixture.scope.deadline().unwrap();
    let child = ScopedWaitableChild::spawn_before_deadline(
        &mut command(input, CREDENTIAL),
        deadline.into_std(),
    )
    .unwrap();
    issuer.bind_child(&child).unwrap();
    assert!(issuer.serve().await.unwrap().adopted);
    child
        .wait_for_clean_exit(deadline.into_std())
        .await
        .unwrap();
}

use super::tests::Fixture;
use super::*;
use crate::fixture_constants as fixture;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use yolo_process::ProcessIdentity;

const HELPER_FILTER: &str = "scavenge::containment_tests::broker_containment_helper";
const HELPER_ROLE_ENVIRONMENT: &str = "YOLO_BROKER_CONTAINMENT_TEST_ROLE";
const HELPER_ROLE: &str = "broker";
const HELPER_LIFETIME: Duration = Duration::from_secs(30);

#[test]
fn orphan_broker_is_exactly_contained_before_record_removal() {
    let fixture = Fixture::new();
    let authority = fixture.family.authority.to_string();
    let mut child = spawn_helper(Some(&authority));
    publish_broker(&fixture, ProcessIdentity::capture(child.id()).unwrap());

    run(&fixture.store).unwrap();

    child.wait().unwrap();
    assert!(fixture.record_missing());
}

#[test]
fn forged_record_cannot_mint_broker_containment_authority() {
    let fixture = Fixture::new();
    let mut child = spawn_helper(None);
    publish_broker(&fixture, ProcessIdentity::capture(child.id()).unwrap());

    assert!(run(&fixture.store).is_err());
    assert!(child.try_wait().unwrap().is_none());
    assert!(!fixture.record_missing());
    child.kill().unwrap();
    child.wait().unwrap();
}

#[test]
fn broker_containment_helper() {
    if std::env::var(HELPER_ROLE_ENVIRONMENT).as_deref() == Ok(HELPER_ROLE) {
        std::thread::sleep(HELPER_LIFETIME);
    }
}

fn spawn_helper(authority: Option<&str>) -> Child {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            fixture::TEST_HARNESS_EXACT_ARGUMENT,
            HELPER_FILTER,
            fixture::TEST_HARNESS_NOCAPTURE_ARGUMENT,
        ])
        .env(HELPER_ROLE_ENVIRONMENT, HELPER_ROLE)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(authority) = authority {
        command.env(fixture::BROKER_SUPERVISION_AUTHORITY_ENVIRONMENT, authority);
    }
    command.spawn().unwrap()
}

fn publish_broker(fixture: &Fixture, broker: ProcessIdentity) {
    let mut record = fixture.intent(exited_identity());
    record.activate(broker).unwrap();
    fixture
        .store
        .write(&fixture.family.record, &record)
        .unwrap();
}

fn exited_identity() -> ProcessIdentity {
    let mut child = Command::new(fixture::TEST_EXIT_EXECUTABLE).spawn().unwrap();
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    child.wait().unwrap();
    identity
}

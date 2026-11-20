use super::{constants::*, fixture::*};
use yolo_hub_listener_binding::ListenerBindings;

#[test]
fn allocation_survives_reopen_and_initial_interruption_without_rewriting_configuration() {
    let fixture = Fixture::new(true);
    let before = fixture.configuration.current().unwrap();
    let bindings = fixture.open(true).unwrap();
    assert_eq!(bindings.address(&before).unwrap(), None);
    assert!(bindings.verify(&before).is_err());
    drop(bindings);
    let mut bindings = fixture.open(false).unwrap();
    bindings
        .establish(&before, ALLOCATED.parse().unwrap())
        .unwrap();
    assert!(
        bindings
            .establish(&before, NEXT_ALLOCATED.parse().unwrap())
            .is_err()
    );
    bindings.verify(&before).unwrap();
    let bytes = std::fs::read(fixture.path()).unwrap();
    drop(bindings);
    let bindings = fixture.open(false).unwrap();
    assert_eq!(
        bindings.address(&before).unwrap(),
        Some(ALLOCATED.parse().unwrap())
    );
    assert_eq!(std::fs::read(fixture.path()).unwrap(), bytes);
    assert_eq!(fixture.configuration.current().unwrap(), before);
}

#[test]
fn existing_ephemeral_configuration_cannot_recreate_a_missing_binding() {
    let fixture = Fixture::new(true);
    assert!(fixture.open(false).is_err());
    assert!(!fixture.path().exists());
    let before = fixture.configuration.current().unwrap();
    let mut bindings = fixture.open(true).unwrap();
    assert!(
        bindings
            .establish(&before, WRONG_IP.parse().unwrap())
            .is_err()
    );
    assert!(
        bindings
            .establish(&before, LOOPBACK.parse().unwrap())
            .is_err()
    );
    bindings
        .establish(&before, ALLOCATED.parse().unwrap())
        .unwrap();
    assert!(fixture.open(false).is_err());
    drop(bindings);
    std::fs::rename(fixture.path(), fixture.directory.path().join(SAVED)).unwrap();
    assert!(fixture.open(false).is_err());
    assert!(!fixture.path().exists());
}

#[test]
fn saved_binding_rejects_wrong_hub_identity_and_unrelated_configuration() {
    let fixture = Fixture::new(false);
    drop(fixture.open(false).unwrap());
    let foreign = Fixture::new(false);
    assert!(
        ListenerBindings::open(
            fixture.hub.directory().unwrap(),
            foreign.hub.peer_identity(),
            &foreign.configuration.current().unwrap(),
            false,
        )
        .is_err()
    );
    let bindings = fixture.open(false).unwrap();
    let mut unrelated = fixture.configuration.current().unwrap();
    unrelated.last_change = uuid::Uuid::new_v4();
    assert!(bindings.address(&unrelated).is_err());
    assert!(
        bindings
            .verify(&fixture.configuration.current().unwrap())
            .is_err()
    );
}

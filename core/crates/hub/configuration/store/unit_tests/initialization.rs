use super::{constants::*, fixtures::*};

#[test]
fn initialization_evidence_is_durable_monotonic_and_does_not_change_public_state() {
    let fixture = Fixture::new();
    let mut store = fixture.open(None).unwrap();
    let original = store.current().unwrap();
    assert!(!store.listener_binding_initialized().unwrap());
    store.complete_listener_initialization(|| Ok(())).unwrap();
    assert!(store.listener_binding_initialized().unwrap());
    assert_eq!(store.current().unwrap(), original);
    let bytes = fixture.bytes();
    store.complete_listener_initialization(|| Ok(())).unwrap();
    assert_eq!(fixture.bytes(), bytes);
    drop(store);
    let mut store = fixture.open(None).unwrap();
    assert!(store.listener_binding_initialized().unwrap());
    assert_eq!(store.current().unwrap(), original);
    let update = change(&original);
    let expected = store.planned(&update).unwrap();
    assert_eq!(store.apply(update).unwrap(), expected);
    assert!(store.listener_binding_initialized().unwrap());
    assert!(fixture.record().listener_binding_initialized);
}

#[test]
fn lost_listener_commit_fence_cannot_mark_first_startup_initialized() {
    let fixture = Fixture::new();
    let mut store = fixture.open(None).unwrap();
    let bytes = fixture.bytes();
    let mut first_check = true;
    assert!(
        store
            .complete_listener_initialization(|| {
                if first_check {
                    first_check = false;
                    Ok(())
                } else {
                    anyhow::bail!(LISTENER_REVOKED)
                }
            })
            .is_err()
    );
    assert_eq!(fixture.bytes(), bytes);
    drop(store);
    assert!(
        !fixture
            .open(None)
            .unwrap()
            .listener_binding_initialized()
            .unwrap()
    );
}

#[test]
fn old_or_truncated_private_record_cannot_gain_first_startup_authority() {
    let fixture = Fixture::new();
    let mut store = fixture.open(None).unwrap();
    store.complete_listener_initialization(|| Ok(())).unwrap();
    let mut record = serde_json::to_value(fixture.record()).unwrap();
    drop(store);
    record
        .as_object_mut()
        .unwrap()
        .remove(LISTENER_INITIALIZED_FIELD);
    fixture.replace(&record);
    let bytes = fixture.bytes();
    assert!(fixture.open(None).is_err());
    assert_eq!(fixture.bytes(), bytes);
}

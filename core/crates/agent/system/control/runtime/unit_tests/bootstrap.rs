use super::fixture::*;
use crate::{FreshInitialization, JournalError};
use yolo_terminal_model::{BootstrapDelivery, RuntimeRecord};

#[test]
fn only_a_settled_serviceable_bootstrap_can_initialize() {
    for delivery in [
        BootstrapDelivery::NotRequired,
        BootstrapDelivery::Delivered,
        BootstrapDelivery::Survivor,
    ] {
        let mut fixture = Fixture::new();
        fixture.runtime.bootstrap_delivery = delivery;
        fixture.proof().unwrap();
    }
    for delivery in [
        BootstrapDelivery::Pending,
        BootstrapDelivery::Failed,
        BootstrapDelivery::Unknown,
    ] {
        let fixture = Fixture::new();
        let mut current = fixture.runtime.clone();
        current.bootstrap_delivery = delivery;
        assert!(matches!(
            FreshInitialization::capture(fixture.session(), &fixture.paths, &current),
            Err(JournalError::ScopeMismatch)
        ));
    }
}

#[test]
fn an_already_published_ready_runtime_is_not_fresh_initialization_evidence() {
    let fixture = Fixture::new();
    let path = fixture.paths.runtime_record(fixture.runtime.terminal_id);
    let mut durable: RuntimeRecord = fixture.read(&path);
    durable.bootstrap_delivery = BootstrapDelivery::NotRequired;
    fixture.write(&path, &durable);
    assert!(matches!(fixture.proof(), Err(JournalError::ScopeMismatch)));
    assert!(!fixture.anchor().exists());
    assert!(!fixture.journal().exists());
}

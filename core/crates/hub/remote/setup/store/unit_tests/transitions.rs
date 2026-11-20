use super::fixture::Fixture;
use yolo_hub_remote_setup_store::{OpenedSetup, SetupOperation, SetupPhase};
use yolo_remote_provisioning_protocol::{ProvisionOutcome, ProvisionStatus};

#[test]
fn only_prepared_can_create_a_fresh_journal_and_valid_steps_are_durable() {
    let fixture = Fixture::new();
    for phase in [
        SetupPhase::Provisioning {
            broker_directory: fixture.broker,
        },
        fixture.routing(),
        fixture.complete(),
    ] {
        let mut forged = fixture.record.clone();
        forged.phase = phase;
        assert!(SetupOperation::open(fixture.parent.try_clone().unwrap(), forged).is_err());
        assert!(!fixture.file().exists());
    }
    let mut operation = fixture.fresh();
    assert_eq!(operation.record().broker_directory(), None);
    assert!(operation.transition(fixture.routing()).is_err());
    operation.verify().unwrap();
    operation
        .transition(SetupPhase::Provisioning {
            broker_directory: fixture.broker,
        })
        .unwrap();
    assert_eq!(operation.record().broker_directory(), Some(fixture.broker));
    assert!(operation.transition(SetupPhase::Prepared {}).is_err());
    operation.transition(fixture.routing()).unwrap();
    assert!(
        operation
            .transition(SetupPhase::Provisioning {
                broker_directory: fixture.broker
            })
            .is_err()
    );
    operation.transition(fixture.complete()).unwrap();
    assert!(operation.transition(fixture.complete()).is_err());
    operation.verify().unwrap();
    drop(operation);
    let OpenedSetup::Existing(reopened) = fixture.open() else {
        panic!("completed journal was recreated")
    };
    assert_eq!(reopened.record().phase, fixture.complete());
}

#[test]
fn successor_phases_cannot_rebind_broker_directory_or_replace_the_routing_evidence() {
    let fixture = Fixture::new();
    let mut operation = fixture.fresh();
    operation
        .transition(SetupPhase::Provisioning {
            broker_directory: fixture.broker,
        })
        .unwrap();
    let mut wrong = fixture.routing();
    let SetupPhase::Routing {
        broker_directory, ..
    } = &mut wrong
    else {
        unreachable!()
    };
    *broker_directory = fixture.other_directory();
    assert!(operation.transition(wrong).is_err());
    operation.transition(fixture.routing()).unwrap();
    let mut wrong = fixture.complete();
    let SetupPhase::Complete {
        broker_directory, ..
    } = &mut wrong
    else {
        unreachable!()
    };
    *broker_directory = fixture.other_directory();
    assert!(operation.transition(wrong).is_err());
    let mut wrong = fixture.complete();
    let SetupPhase::Complete { change, .. } = &mut wrong else {
        unreachable!()
    };
    *change = None;
    assert!(operation.transition(wrong).is_err());
    let mut wrong = fixture.complete();
    let SetupPhase::Complete { provision, .. } = &mut wrong else {
        unreachable!()
    };
    let ProvisionOutcome::Ready {
        publication,
        receipt,
        enrollment,
    } = &provision.fields().outcome
    else {
        unreachable!()
    };
    let mut fields = enrollment.fields().clone();
    fields.configuration_revision = super::constants::NEXT_REVISION;
    *provision = ProvisionStatus::new(
        fixture.record.request.request.clone(),
        ProvisionOutcome::Ready {
            publication: publication.clone(),
            receipt: receipt.clone(),
            enrollment: yolo_remote_provisioning_protocol::RemoteBootstrapReceipt::new(fields)
                .unwrap(),
        },
    )
    .unwrap();
    let mut copied = operation.record().clone();
    copied.phase = wrong.clone();
    copied.validate().unwrap(); // Independently valid evidence still cannot replace the admitted evidence.
    assert!(operation.transition(wrong).is_err());
    operation.verify().unwrap();
    operation.transition(fixture.complete()).unwrap();
}

#[test]
fn no_change_route_retains_explicit_none_through_completion() {
    let mut fixture = Fixture::new();
    let SetupPhase::Routing {
        change: Some(change),
        ..
    } = fixture.routing()
    else {
        unreachable!()
    };
    fixture.record.baseline.state.configuration = change.configuration;
    fixture.record.validate().unwrap();
    let routing = fixture.routing();
    assert!(matches!(&routing, SetupPhase::Routing { change: None, .. }));
    let mut operation = fixture.fresh();
    operation
        .transition(SetupPhase::Provisioning {
            broker_directory: fixture.broker,
        })
        .unwrap();
    operation.transition(routing).unwrap();
    operation.transition(fixture.complete()).unwrap();
    assert!(matches!(
        &operation.record().phase,
        SetupPhase::Complete { change: None, .. }
    ));
}

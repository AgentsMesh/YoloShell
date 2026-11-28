use super::{constants::*, fixture::Fixture};
use yolo_hub_remote_setup_store::{OpenedSetup, SetupOperation, SetupPhase};
use yolo_remote_provisioning_protocol::ProvisionRequest;

#[test]
fn same_request_id_cannot_switch_profile_source_remote_home_or_registration() {
    let fixture = Fixture::new();
    drop(fixture.fresh());
    let before = std::fs::read(fixture.file()).unwrap();
    let mut wrong = fixture.record.clone();
    let mut fields = wrong.request.request.fields().clone();
    fields.host_home = "/srv/different-home".into();
    wrong.request.request = ProvisionRequest::new(fields).unwrap();
    assert!(SetupOperation::open(fixture.parent.try_clone().unwrap(), wrong).is_err());
    let mut wrong = fixture.record.clone();
    let mut fields = wrong.request.request.fields().clone();
    fields.source = yolo_distribution_bundle::BundleSource::new(
        OTHER_SOURCE.into(),
        fields.source.descriptor().clone(),
    )
    .unwrap();
    wrong.request.request = ProvisionRequest::new(fields).unwrap();
    assert!(SetupOperation::open(fixture.parent.try_clone().unwrap(), wrong).is_err());
    let mut wrong = fixture.record.clone();
    wrong.registration.broker_state = fixture.root.path().join(FOREIGN_NAME);
    assert!(SetupOperation::open(fixture.parent.try_clone().unwrap(), wrong).is_err());
    assert_eq!(std::fs::read(fixture.file()).unwrap(), before);
}

const OTHER_SOURCE: &str = "/srv/other-bundle-source";

#[test]
fn reopening_provisioning_never_reverts_to_prepared_or_repeats_the_submission_transition() {
    let fixture = Fixture::new();
    let mut operation = fixture.fresh();
    operation
        .transition(SetupPhase::Provisioning {
            broker_directory: fixture.broker,
        })
        .unwrap();
    let before = std::fs::read(fixture.file()).unwrap();
    drop(operation);
    let OpenedSetup::Existing(mut existing) = fixture.open() else {
        panic!("unknown provisioning was recreated")
    };
    assert!(existing.transition(SetupPhase::Prepared {}).is_err());
    assert!(
        existing
            .transition(SetupPhase::Provisioning {
                broker_directory: fixture.broker
            })
            .is_err()
    );
    existing.verify().unwrap();
    assert_eq!(std::fs::read(fixture.file()).unwrap(), before);
}

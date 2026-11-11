use super::{constants::*, fixture::Fixture};
use yolo_hub_remote_setup_store::{SetupPhase, SetupRecord};
use yolo_remote_provisioning_protocol::{ProvisionOutcome, ProvisionPhase, ProvisionStatus};

#[test]
fn persisted_setup_binds_owner_profile_request_and_baseline_revision() {
    let fixture = Fixture::new();
    fixture.record.validate().unwrap();
    let mut changes = Vec::new();
    let mut record = fixture.record.clone();
    record.schema_version = u16::MAX;
    changes.push(record);
    let mut record = fixture.record.clone();
    record.owner = record
        .request
        .request
        .fields()
        .bootstrap
        .fields()
        .expected_peer
        .unwrap();
    changes.push(record);
    let mut record = fixture.record.clone();
    record.baseline.applied = false;
    changes.push(record);
    let mut record = fixture.record.clone();
    record.baseline.state.revision = NEXT_REVISION;
    changes.push(record);
    let mut record = fixture.record.clone();
    record.registration.config_revision = "f".repeat(HASH_BYTES);
    changes.push(record);
    for record in changes {
        assert!(record.validate().is_err());
    }
}

#[test]
fn routing_cannot_claim_ready_from_unknown_provision_or_an_unrelated_configuration() {
    let fixture = Fixture::new();
    for outcome in [
        ProvisionOutcome::Absent {},
        ProvisionOutcome::Unknown {
            phase: ProvisionPhase::Enrolling,
        },
        ProvisionOutcome::Superseded {
            phase: ProvisionPhase::Complete,
        },
    ] {
        let mut record = fixture.record.clone();
        record.phase = SetupPhase::Routing {
            broker_directory: fixture.broker,
            change: None,
            provision: ProvisionStatus::new(record.request.request.clone(), outcome).unwrap(),
        };
        assert!(record.validate().is_err());
    }
    let mut record = fixture.record.clone();
    record.phase = fixture.routing();
    record.validate().unwrap();
    let SetupPhase::Routing {
        change: Some(change),
        ..
    } = &mut record.phase
    else {
        panic!("expected link change")
    };
    change.configuration.listen = Some((std::net::Ipv4Addr::LOCALHOST, EXTRA_PORT).into());
    assert!(record.validate().is_err());
}

#[test]
fn journal_wire_rejects_unknown_fields_and_missing_phase() {
    let fixture = Fixture::new();
    let value = serde_json::to_value(&fixture.record).unwrap();
    assert_eq!(
        serde_json::from_value::<SetupRecord>(value.clone()).unwrap(),
        fixture.record
    );
    let mut invalid = value.clone();
    invalid[EXTRA_FIELD] = serde_json::Value::Bool(true);
    assert!(serde_json::from_value::<SetupRecord>(invalid).is_err());
    let mut invalid = value;
    invalid.as_object_mut().unwrap().remove(PHASE_FIELD);
    assert!(serde_json::from_value::<SetupRecord>(invalid).is_err());
}

const HASH_BYTES: usize = 64;
const EXTRA_PORT: u16 = 32123;

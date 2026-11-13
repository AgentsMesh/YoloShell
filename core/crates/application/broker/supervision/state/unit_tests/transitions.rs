use crate::{
    BrokerReadiness, SupervisionPhase, SupervisionRecord, fixture_constants::*, launch_fixture,
};
use yolo_process::ProcessIdentity;

fn intent() -> (SupervisionRecord, ProcessIdentity) {
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    (
        SupervisionRecord::intent(&launch_fixture::launch(), identity, identity).unwrap(),
        identity,
    )
}

fn readiness() -> BrokerReadiness {
    BrokerReadiness {
        instance_id: uuid::Uuid::new_v4(),
        host_id: HOST_ID.into(),
        config_revision: CONFIG_REVISION.into(),
    }
}

#[test]
fn named_transitions_preserve_identity_and_reject_skipping_admission_phases() {
    let (mut record, identity) = intent();
    let original = serde_json::to_vec(&record).unwrap();
    assert!(record.ready(&readiness()).is_err());
    assert!(record.mark_running().is_err());
    assert!(record.begin_termination().is_err());
    assert!(record.mark_settled().is_err());
    assert_eq!(serde_json::to_vec(&record).unwrap(), original);
    record.activate(identity).unwrap();
    let active = record.clone();
    assert!(record.activate(identity).is_err());
    assert!(record.mark_running().is_err());
    record.ready(&readiness()).unwrap();
    assert!(active.same_custody(&record));
    record.mark_running().unwrap();
    assert_eq!(record.phase(), SupervisionPhase::Running);
    record.begin_termination().unwrap();
    record.begin_termination().unwrap();
    record.mark_settled().unwrap();
    record.mark_settled().unwrap();
    assert_eq!(record.phase(), SupervisionPhase::Settled);
    assert!(record.activate(identity).is_err());
    assert!(record.mark_running().is_err());
    assert_eq!(record.launcher_identity(), identity);
    assert_eq!(record.supervisor_identity(), identity);
    assert_eq!(record.broker_identity(), Some(identity));
}

#[test]
fn bad_readiness_cannot_modify_an_active_record_or_publish_a_different_generation() {
    let (mut record, identity) = intent();
    record.activate(identity).unwrap();
    let original = serde_json::to_vec(&record).unwrap();
    for invalid in [
        BrokerReadiness {
            instance_id: uuid::Uuid::nil(),
            ..readiness()
        },
        BrokerReadiness {
            host_id: String::new(),
            ..readiness()
        },
        BrokerReadiness {
            config_revision: String::new(),
            ..readiness()
        },
        BrokerReadiness {
            config_revision: "a".repeat(CONFIG_REVISION.len()),
            ..readiness()
        },
    ] {
        assert!(record.ready(&invalid).is_err());
        assert_eq!(serde_json::to_vec(&record).unwrap(), original);
    }
    record.mark_settled().unwrap();
    assert_eq!(record.phase(), SupervisionPhase::Settled);
}

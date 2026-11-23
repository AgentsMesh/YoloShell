use crate::{
    BrokerReadiness, ChildLaunch, SupervisionPaths, SupervisionRecord, fixture_constants::*,
    io_kind, launch_fixture,
};
use yolo_process::ProcessIdentity;
use yolo_remote_broker::BrokerPaths;

#[test]
fn anchored_record_and_request_roundtrip_preserves_exact_custody_until_guarded_removal() {
    let root = tempfile::tempdir().unwrap();
    let broker_paths = BrokerPaths::new(root.path().join("state")).unwrap();
    let launch = launch_fixture::for_paths(&broker_paths, uuid::Uuid::new_v4());
    let paths = SupervisionPaths::for_launch(&launch).unwrap();
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    let mut record = SupervisionRecord::intent(&launch, identity, identity).unwrap();
    paths.write_record(&record).unwrap();
    let loaded = paths.read_record().unwrap();
    loaded.verify(&launch, identity, identity).unwrap();
    assert_eq!(
        serde_json::to_vec(&loaded).unwrap(),
        serde_json::to_vec(&record).unwrap()
    );
    assert!(!paths.adoption_present(&record).unwrap());
    assert!(paths.write_adoption(&record).is_err());
    record.activate(identity).unwrap();
    record
        .ready(&BrokerReadiness {
            instance_id: uuid::Uuid::new_v4(),
            host_id: HOST_ID.into(),
            config_revision: CONFIG_REVISION.into(),
        })
        .unwrap();
    paths.write_record(&record).unwrap();
    paths.write_adoption(&record).unwrap();
    paths.write_termination(&record).unwrap();
    assert!(paths.adoption_present(&record).unwrap());
    assert!(paths.termination_present(&record).unwrap());
    paths.remove_adoption().unwrap();
    assert!(!paths.adoption_present(&record).unwrap());
    paths.remove_requests().unwrap();
    assert!(!paths.termination_present(&record).unwrap());
    assert!(record.same_custody(&paths.read_record().unwrap()));
    paths.remove_all().unwrap();
    assert_eq!(
        io_kind(&paths.read_record().unwrap_err()),
        Some(std::io::ErrorKind::NotFound)
    );
}

#[test]
fn the_launch_description_cannot_rebind_paths_to_a_different_state_anchor() {
    let root = tempfile::tempdir().unwrap();
    let first = BrokerPaths::new(root.path().join("first")).unwrap();
    let second = BrokerPaths::new(root.path().join("second")).unwrap();
    let original = launch_fixture::for_paths(&first, uuid::Uuid::new_v4());
    let wrong = ChildLaunch::new(
        original.config().into(),
        original.state_dir().into(),
        original.config_revision().into(),
        second.state_anchor(),
        original.authority(),
        original.startup_deadline().clone(),
    )
    .unwrap();
    assert!(SupervisionPaths::for_launch(&wrong).is_err());
    assert!(SupervisionPaths::for_launch(&original).is_ok());
}

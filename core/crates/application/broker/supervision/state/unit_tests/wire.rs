use crate::{SupervisionRecord, fixture_constants::*, launch_fixture};
use yolo_process::ProcessIdentity;

#[test]
fn moving_records_keeps_the_exact_existing_json_schema_and_field_order() {
    let launch = launch_fixture::launch();
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    let record = SupervisionRecord::intent(&launch, identity, identity).unwrap();
    let expected = format!(
        concat!(
            "{{\"version\":{},\"launch_id\":\"{}\",\"generation\":\"{}\",",
            "\"boot_id\":\"{}\",\"supervisor\":{{\"pid\":{},\"start_time\":{}}},",
            "\"launcher\":{{\"pid\":{},\"start_time\":{}}},\"broker\":null,",
            "\"instance_id\":null,\"broker_host_id\":null,\"phase\":\"intent\"}}"
        ),
        yolo_remote_broker::BROKER_SUPERVISION_RECORD_VERSION,
        launch.authority(),
        CONFIG_REVISION,
        yolo_process::boot_id(),
        identity.pid,
        identity.start_time,
        identity.pid,
        identity.start_time
    );
    assert_eq!(serde_json::to_vec(&record).unwrap(), expected.as_bytes());
    let decoded: SupervisionRecord = serde_json::from_str(&expected).unwrap();
    decoded.verify(&launch, identity, identity).unwrap();
    assert!(record.same_custody(&decoded));
    let mut extra = serde_json::to_value(&record).unwrap();
    extra[UNEXPECTED_FIELD] = true.into();
    assert!(serde_json::from_value::<SupervisionRecord>(extra).is_err());
}

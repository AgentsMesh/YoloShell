use super::*;
use crate::fixture_constants::CONFIG_REVISION as TEST_REVISION;
use crate::{BrokerReadiness, SupervisionRecord, launch_fixture::launch as options};
use yolo_process::ProcessIdentity;
const TEST_HOST: &str = "host";

fn ready_record() -> SupervisionRecord {
    let options = options();
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    let mut record = SupervisionRecord::intent(&options, identity, identity).unwrap();
    record.activate(identity).unwrap();
    record
        .ready(&BrokerReadiness {
            instance_id: uuid::Uuid::new_v4(),
            host_id: TEST_HOST.into(),
            config_revision: TEST_REVISION.into(),
        })
        .unwrap();
    record
}

#[test]
fn adoption_request_rejects_preseeded_intent_shape() {
    let options = options();
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    let intent = SupervisionRecord::intent(&options, identity, identity).unwrap();
    assert!(AdoptionRequest::for_record(&intent).is_err());
    let broad = CustodyRequest::for_record(&intent);
    let encoded = serde_json::to_value(broad).unwrap();
    assert!(serde_json::from_value::<AdoptionRequest>(encoded).is_err());
}

#[test]
fn adoption_request_binds_the_complete_private_readiness() {
    let record = ready_record();
    let mut request = AdoptionRequest::for_record(&record).unwrap();
    assert!(request.verify(&record).is_ok());
    request.instance_id = uuid::Uuid::new_v4();
    assert!(request.verify(&record).is_err());
}

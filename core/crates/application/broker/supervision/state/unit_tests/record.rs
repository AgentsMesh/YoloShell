use super::*;
use crate::launch_fixture::launch as options;

fn identities() -> (ProcessIdentity, ProcessIdentity) {
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    (identity, identity)
}

#[test]
fn phase_verification_rejects_fields_from_other_phases() {
    let options = options();
    let (supervisor, launcher) = identities();
    let mut intent = SupervisionRecord::intent(&options, supervisor, launcher).unwrap();
    assert!(intent.verify(&options, supervisor, launcher).is_ok());

    intent.broker = Some(supervisor.into());
    assert!(intent.verify(&options, supervisor, launcher).is_err());
    assert!(intent.activate(supervisor).is_err());
    intent.broker = None;
    intent.activate(supervisor).unwrap();
    intent.instance_id = Some(uuid::Uuid::new_v4());
    assert!(intent.verify(&options, supervisor, launcher).is_err());
}

#[test]
fn ready_phase_requires_complete_non_nil_readiness() {
    let options = options();
    let (supervisor, launcher) = identities();
    let mut record = SupervisionRecord::intent(&options, supervisor, launcher).unwrap();
    record.activate(supervisor).unwrap();
    record.set_phase(SupervisionPhase::Ready);
    assert!(record.verify(&options, supervisor, launcher).is_err());

    record.instance_id = Some(uuid::Uuid::new_v4());
    record.broker_host_id = Some("host".into());
    assert!(record.verify(&options, supervisor, launcher).is_ok());
}

#[test]
fn verification_binds_both_process_identities() {
    let options = options();
    let (supervisor, launcher) = identities();
    let record = SupervisionRecord::intent(&options, supervisor, launcher).unwrap();
    let wrong = ProcessIdentity {
        pid: supervisor.pid,
        start_time: supervisor.start_time.saturating_add(1),
    };
    assert!(record.verify(&options, wrong, launcher).is_err());
    assert!(record.verify(&options, supervisor, wrong).is_err());
}

#[path = "record_custody.rs"]
mod custody;

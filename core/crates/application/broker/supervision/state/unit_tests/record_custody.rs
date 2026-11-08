use super::*;

const FIELD_INCREMENT: u64 = 1;
const VERSION_INCREMENT: u16 = 1;
const PID_INCREMENT: u32 = 1;
const DIFFERENT_VALUE: &str = "different-custody-value";
const BROKER_HOST: &str = "original-custody-host";
const CONFIG_REVISION: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[test]
fn custody_matches_each_phase_with_an_exact_valid_record() {
    let (supervisor, launcher) = identities();
    let mut record = SupervisionRecord::intent(&custody_options(), supervisor, launcher).unwrap();
    record.verify_recovery(&record.launch_id).unwrap();
    assert!(record.same_custody(&record));
    record.activate(supervisor).unwrap();
    assert!(record.same_custody(&record));
    record.ready(&readiness()).unwrap();
    for phase in [
        SupervisionPhase::Ready,
        SupervisionPhase::Running,
        SupervisionPhase::Terminating,
        SupervisionPhase::Settled,
    ] {
        record.set_phase(phase);
        record.verify_recovery(&record.launch_id).unwrap();
        assert!(record.same_custody(&record));
    }
}

#[test]
fn custody_accepts_readiness_enrichment_but_rejects_stripping_and_rebinding() {
    let (supervisor, launcher) = identities();
    let mut active = SupervisionRecord::intent(&custody_options(), supervisor, launcher).unwrap();
    active.activate(supervisor).unwrap();
    let mut ready = active.clone();
    ready.ready(&readiness()).unwrap();
    assert!(active.same_custody(&ready));
    assert!(!ready.same_custody(&active));
    let mut different = ready.clone();
    different.instance_id = Some(uuid::Uuid::new_v4());
    assert!(!ready.same_custody(&different));
    different = ready.clone();
    different.broker_host_id = Some(DIFFERENT_VALUE.into());
    assert!(!ready.same_custody(&different));
}

#[test]
fn custody_rejects_invalid_expected_and_observed_phase_shapes() {
    let (supervisor, launcher) = identities();
    let valid = SupervisionRecord::intent(&custody_options(), supervisor, launcher).unwrap();
    let mut malformed = valid.clone();
    malformed.broker = Some(supervisor.into());
    assert!(!malformed.same_custody(&valid));
    assert!(!valid.same_custody(&malformed));
}

#[test]
fn custody_rejects_each_independent_fixed_identity_mutation() {
    let (supervisor, launcher) = identities();
    let mut expected = SupervisionRecord::intent(&custody_options(), supervisor, launcher).unwrap();
    expected.activate(supervisor).unwrap();
    expected.ready(&readiness()).unwrap();
    let mutations: &[fn(&mut SupervisionRecord)] = &[
        |value| value.version += VERSION_INCREMENT,
        |value| value.launch_id = uuid::Uuid::new_v4().to_string(),
        |value| value.generation = DIFFERENT_VALUE.into(),
        |value| value.boot_id = DIFFERENT_VALUE.into(),
        |value| value.supervisor.pid += PID_INCREMENT,
        |value| value.supervisor.start_time += FIELD_INCREMENT,
        |value| value.launcher.pid += PID_INCREMENT,
        |value| value.launcher.start_time += FIELD_INCREMENT,
        |value| value.broker.as_mut().unwrap().pid += PID_INCREMENT,
        |value| value.broker.as_mut().unwrap().start_time += FIELD_INCREMENT,
    ];
    for mutate in mutations {
        let mut current = expected.clone();
        mutate(&mut current);
        assert!(current.shape_valid());
        assert!(!expected.same_custody(&current));
        assert!(!current.same_custody(&expected));
    }
}

fn readiness() -> crate::BrokerReadiness {
    crate::BrokerReadiness {
        instance_id: uuid::Uuid::new_v4(),
        host_id: BROKER_HOST.into(),
        config_revision: CONFIG_REVISION.into(),
    }
}

fn custody_options() -> ChildLaunch {
    options()
}

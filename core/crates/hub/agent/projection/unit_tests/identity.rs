use super::{
    constants::*,
    fixture::record,
    view::{exit, view},
};
use crate::identity::{process, same_execution, select, subject};
use yolo_agent_contract::AgentState;
use yolo_hub_identity::{AgentId, RuntimeEpoch};
use yolo_terminal_model::RecoveryState;

#[test]
fn logical_identity_reuses_exact_subject_and_process_but_not_parallel_native_reference() {
    let source = view();
    let record = record(&source);
    let binding = source.agent.as_ref().unwrap();
    let subject = subject(binding).unwrap();
    let id = record.latest[&subject.terminal_id];
    let entry = &record.entries[&id];
    assert_eq!(
        select(&record, &subject, entry.runtime, binding, &source).unwrap(),
        Some(id)
    );
    assert!(same_execution(entry, entry.runtime, binding));
    let mut changed = binding.clone();
    changed.process_start_time = Some(PROVIDER_START + IDENTITY_STEP);
    assert_eq!(
        select(&record, &subject, entry.runtime, &changed, &source).unwrap(),
        None
    );
    assert!(!same_execution(entry, entry.runtime, &changed));
    changed.process_start_time = binding.process_start_time;
    changed.process_pid = Some(PROVIDER_PID + IDENTITY_STEP as u32);
    assert_eq!(
        select(&record, &subject, entry.runtime, &changed, &source).unwrap(),
        None
    );
    assert!(!same_execution(entry, RuntimeEpoch::new(), binding));
}

#[test]
fn historical_exit_keeps_execution_but_relaunch_changes_it() {
    let mut source = view();
    let mut record = record(&source);
    let id = *record.entries.keys().next().unwrap();
    let runtime = record.entries[&id].runtime;
    exit(&mut source);
    let binding = source.agent.as_ref().unwrap();
    let subject = subject(binding).unwrap();
    assert_eq!(
        select(&record, &subject, runtime, binding, &source).unwrap(),
        Some(id)
    );
    assert!(same_execution(&record.entries[&id], runtime, binding));
    assert_eq!(process(binding), None);
    record.entries.get_mut(&id).unwrap().source_exited = true;
    let mut relaunched = binding.clone();
    relaunched.state = AgentState::Running;
    relaunched.process_pid = Some(PROVIDER_PID);
    relaunched.process_start_time = Some(PROVIDER_START);
    assert!(select(&record, &subject, runtime, &relaunched, &source).is_err());
    relaunched.process_start_time = Some(PROVIDER_START + IDENTITY_STEP);
    assert_eq!(
        select(&record, &subject, runtime, &relaunched, &source).unwrap(),
        Some(id)
    );
    assert!(!same_execution(&record.entries[&id], runtime, &relaunched));
}

#[test]
fn only_explicit_host_resume_links_across_terminal_executions() {
    let mut source = view();
    let record = record(&source);
    let binding = source.agent.as_ref().unwrap().clone();
    let subject = subject(&binding).unwrap();
    let runtime = RuntimeEpoch::new();
    assert_eq!(
        select(&record, &subject, runtime, &binding, &source).unwrap(),
        None
    );
    source.manifest.recovery_state = RecoveryState::Resumed;
    assert_eq!(
        select(&record, &subject, runtime, &binding, &source).unwrap(),
        Some(record.latest[&subject.terminal_id])
    );
    source.manifest.recovery_state = RecoveryState::Recreated;
    assert_eq!(
        select(&record, &subject, runtime, &binding, &source).unwrap(),
        None
    );
}

#[test]
fn invalid_and_regressed_binding_metadata_cannot_retarget_existing_record() {
    let source = view();
    let mut record = record(&source);
    let binding = source.agent.as_ref().unwrap();
    let subject = subject(binding).unwrap();
    let runtime = record.entries.values().next().unwrap().runtime;
    let mut stale = binding.clone();
    stale.updated_at_ms -= IDENTITY_STEP as i64;
    assert!(select(&record, &subject, runtime, &stale, &source).is_err());
    let mut different = subject.clone();
    different.native_reference = CHANGED_PROVIDER_REFERENCE.into();
    assert_eq!(
        select(&record, &different, runtime, binding, &source).unwrap(),
        None
    );
    record.latest.insert(subject.terminal_id, AgentId::new());
    assert!(select(&record, &subject, runtime, binding, &source).is_err());
    record.latest.clear();
    assert_eq!(
        select(&record, &subject, runtime, binding, &source).unwrap(),
        None
    );
    for invalid in [
        String::new(),
        String::from("relative"),
        String::from("/bad\npath"),
        "/".repeat(crate::constants::MAX_PRIVATE_METADATA_BYTES + 1),
    ] {
        let mut changed = binding.clone();
        changed.data_home = invalid;
        assert!(crate::identity::subject(&changed).is_err());
    }
}

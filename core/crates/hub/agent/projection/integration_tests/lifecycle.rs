use super::{
    constants::*,
    fixtures::{directory, open},
    view::{exit, view},
};
use uuid::Uuid;
use yolo_agent_contract::{AgentRecoveryAvailability, AgentState};
use yolo_hub_agent_protocol::{Activity, AgentCapabilities, Freshness, Lifecycle, Reachability};
use yolo_terminal_model::{RecoveryState, TerminalRuntimeStatus};

#[test]
fn identity_survives_restart_while_freshness_must_be_reconfirmed_from_host() {
    let (_root, path, hub) = directory();
    let mut store = open(&path, hub);
    let source = view();
    let first = store.project(&source).unwrap().unwrap();
    assert_eq!(
        first.fields().provider.native_session_reference(),
        Some(PROVIDER_REFERENCE)
    );
    assert_eq!(first.fields().provider.version(), None);
    assert_eq!(first.fields().capabilities, AgentCapabilities::UNSUPPORTED);
    assert_eq!(store.project(&source).unwrap().unwrap(), first);
    assert_eq!(store.hub_id(), hub);
    drop(store);
    let mut reopened = open(&path, hub);
    let cached = reopened
        .get(first.cursor().target().address)
        .unwrap()
        .unwrap();
    assert_eq!(cached.cursor().target(), first.cursor().target());
    assert_eq!(cached.fields().freshness, Freshness::Stale);
    assert_eq!(cached.fields().reachability, Reachability::Unreachable);
    let current = reopened.project(&source).unwrap().unwrap();
    assert_eq!(current.cursor().target(), first.cursor().target());
    assert_eq!(current.fields().freshness, Freshness::Current);
    assert!(current.cursor().sequence() > cached.cursor().sequence());
}

#[test]
fn simultaneous_native_reference_reuse_does_not_alias_agent_and_pid_reuse_changes_execution() {
    let (_root, path, hub) = directory();
    let mut store = open(&path, hub);
    let mut source = view();
    let first = store.project(&source).unwrap().unwrap();
    source.agent.as_mut().unwrap().process_start_time = Some(PROVIDER_START + IDENTITY_STEP);
    let parallel = store.project(&source).unwrap().unwrap();
    assert_ne!(
        parallel.cursor().target().address,
        first.cursor().target().address
    );
    assert_ne!(
        parallel.cursor().target().execution_id,
        first.cursor().target().execution_id
    );
    exit(&mut source);
    let exited = store.project(&source).unwrap().unwrap();
    assert_eq!(exited.cursor().target(), parallel.cursor().target());
    assert_eq!(
        exited.fields().lifecycle,
        Lifecycle::Exited { exit_code: None }
    );
    let binding = source.agent.as_mut().unwrap();
    binding.state = AgentState::Running;
    binding.process_pid = Some(PROVIDER_PID + IDENTITY_STEP as u32);
    binding.process_start_time = Some(PROVIDER_START);
    binding.updated_at_ms += IDENTITY_STEP as i64;
    let relaunched = store.project(&source).unwrap().unwrap();
    assert_eq!(
        relaunched.cursor().target().address,
        parallel.cursor().target().address
    );
    assert_ne!(
        relaunched.cursor().target().execution_id,
        parallel.cursor().target().execution_id
    );
    assert_eq!(store.list_active().unwrap(), vec![relaunched]);
}

#[test]
fn verified_resume_preserves_logical_address_but_recreation_is_another_agent() {
    let (_root, path, hub) = directory();
    let mut store = open(&path, hub);
    let mut source = view();
    let first = store.project(&source).unwrap().unwrap();
    source.runtime.as_mut().unwrap().runtime_epoch = Uuid::new_v4();
    source.manifest.recovery_state = RecoveryState::Resumed;
    let resumed = store.project(&source).unwrap().unwrap();
    assert_eq!(
        first.cursor().target().address,
        resumed.cursor().target().address
    );
    assert_ne!(
        first.cursor().target().execution_id,
        resumed.cursor().target().execution_id
    );
    source.runtime.as_mut().unwrap().runtime_epoch = Uuid::new_v4();
    source.manifest.recovery_state = RecoveryState::Recreated;
    let recreated = store.project(&source).unwrap().unwrap();
    assert_ne!(
        resumed.cursor().target().address,
        recreated.cursor().target().address
    );
}

#[test]
fn absent_or_historical_bindings_cannot_claim_the_current_shell_execution() {
    let (_root, path, hub) = directory();
    let mut store = open(&path, hub);
    let mut historical = view();
    exit(&mut historical);
    assert!(store.project(&historical).unwrap().is_none());
    historical.agent.as_mut().unwrap().state = AgentState::Unknown;
    assert!(store.project(&historical).unwrap().is_none());
    let mut source = view();
    let first = store.project(&source).unwrap().unwrap();
    exit(&mut source);
    source.runtime.as_mut().unwrap().runtime_epoch = Uuid::new_v4();
    let historical = store.project(&source).unwrap().unwrap();
    assert_eq!(historical.fields().terminal, first.fields().terminal);
    assert_eq!(historical.fields().reachability, Reachability::Unreachable);
    source.agent = None;
    assert!(store.project(&source).unwrap().is_none());
    let stale = store.get(first.cursor().target().address).unwrap().unwrap();
    assert_eq!(stale.fields().freshness, Freshness::Stale);
}

#[test]
fn changed_identity_namespaces_and_lost_runtime_do_not_leak_live_state() {
    let (_root, path, hub) = directory();
    let mut store = open(&path, hub);
    let mut source = view();
    let first = store.project(&source).unwrap().unwrap();
    source.runtime.as_mut().unwrap().status = TerminalRuntimeStatus::Unreachable;
    let lost = store.project(&source).unwrap().unwrap();
    assert_eq!(lost.cursor().target(), first.cursor().target());
    assert_eq!(lost.fields().lifecycle, Lifecycle::Unknown {});
    assert_eq!(lost.fields().activity, Activity::Unknown);
    source.runtime.as_mut().unwrap().status = TerminalRuntimeStatus::Running;
    source.agent.as_mut().unwrap().provider_session_id = CHANGED_PROVIDER_REFERENCE.into();
    let changed = store.project(&source).unwrap().unwrap();
    assert_ne!(
        changed.cursor().target().address,
        first.cursor().target().address
    );
    source.agent.as_mut().unwrap().recovery_availability = AgentRecoveryAvailability::Unavailable;
    let unmanaged = store.project(&source).unwrap().unwrap();
    assert_eq!(unmanaged.fields().activity, Activity::Unknown);
}

#[test]
fn separate_shells_never_merge_by_provider_reference_or_process_id() {
    let (_root, path, hub) = directory();
    let mut store = open(&path, hub);
    let first = store.project(&view()).unwrap().unwrap();
    let second = store.project(&view()).unwrap().unwrap();
    assert_ne!(
        first.cursor().target().address,
        second.cursor().target().address
    );
    assert_ne!(
        first.fields().terminal.address,
        second.fields().terminal.address
    );
    assert_eq!(store.list().unwrap().len(), 2);
}

#[test]
fn exited_agent_run_leaves_history_but_no_current_agent_run() {
    let (_root, path, hub) = directory();
    let mut store = open(&path, hub);
    let mut source = view();
    let first = store.project(&source).unwrap().unwrap();
    exit(&mut source);
    let exited = store.project(&source).unwrap().unwrap();
    assert_eq!(exited.cursor().target(), first.cursor().target());
    assert_eq!(
        source.runtime.as_ref().unwrap().status,
        TerminalRuntimeStatus::Running
    );
    assert!(store.list_active().unwrap().is_empty());
    assert_eq!(store.list().unwrap().len(), 1);
    assert_eq!(store.project(&source).unwrap(), Some(exited.clone()));
    assert_eq!(store.list().unwrap(), vec![exited.clone()]);
    assert_eq!(
        store.get(exited.cursor().target().address).unwrap(),
        Some(exited)
    );
}

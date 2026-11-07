use yolo_hub_agent_protocol::*;
use yolo_hub_identity::HubId;

use crate::constants::*;
use crate::fixtures::*;

#[test]
fn snapshots_preserve_origin_and_independent_offline_running_state() {
    let base = fields();
    let snapshot = AgentSnapshot::new(base.clone()).unwrap();
    assert_eq!(snapshot.fields(), &base);
    assert_eq!(snapshot.cursor(), base.cursor);
    assert_eq!(round_trip(&snapshot), snapshot);
    let mut offline = base;
    offline.reachability = Reachability::Unreachable;
    offline.freshness = Freshness::Stale;
    let offline = AgentSnapshot::new(offline).unwrap();
    assert_eq!(offline.fields().lifecycle, Lifecycle::Running {});
    assert_eq!(round_trip(&offline), offline);
}

#[test]
fn snapshot_schema_and_origin_fail_closed() {
    let mut changed = fields();
    changed.schema_version += SEQUENCE_STEP as u16;
    assert_eq!(
        AgentSnapshot::new(changed),
        Err(ProtocolError::UnsupportedSchema)
    );
    let mut changed = fields();
    changed.terminal.address.hub_id = HubId::new();
    assert_eq!(
        AgentSnapshot::new(changed),
        Err(ProtocolError::OriginMismatch)
    );
    let mut wire = serde_json::to_value(AgentSnapshot::new(fields()).unwrap()).unwrap();
    wire[SCHEMA_VERSION_FIELD] = u16::MAX.into();
    assert!(serde_json::from_value::<AgentSnapshot>(wire).is_err());
}

#[test]
fn exited_snapshot_cannot_claim_active_work_or_semantic_control() {
    let mut base = fields();
    base.lifecycle = Lifecycle::Exited { exit_code: None };
    base.activity = Activity::Unknown;
    base.capabilities = AgentCapabilities::UNSUPPORTED;
    for activity in [Activity::Unknown, Activity::Idle] {
        let mut allowed = base.clone();
        allowed.activity = activity;
        assert!(AgentSnapshot::new(allowed).is_ok());
    }
    for activity in [
        Activity::Working,
        Activity::WaitingForInput,
        Activity::WaitingForApproval,
    ] {
        let mut invalid = base.clone();
        invalid.activity = activity;
        assert_eq!(
            AgentSnapshot::new(invalid),
            Err(ProtocolError::InvalidState)
        );
    }
    let mutations: [fn(&mut AgentSnapshotFields); 4] = [
        |s| s.interaction = Some(interaction(s.cursor.target(), InteractionKind::Text)),
        |s| s.capabilities.interrupt = Capability::Available {},
        |s| s.capabilities.submit_prompt = Capability::Available {},
        |s| s.capabilities.respond_interaction = Capability::Available {},
    ];
    for mutate in mutations {
        let mut invalid = base.clone();
        mutate(&mut invalid);
        assert_eq!(
            AgentSnapshot::new(invalid),
            Err(ProtocolError::InvalidState)
        );
    }
}

#[test]
fn pending_interaction_is_bound_to_execution_and_matching_activity() {
    for kind in [
        InteractionKind::Approval,
        InteractionKind::Choice,
        InteractionKind::Text,
    ] {
        let valid = waiting(kind);
        assert_eq!(round_trip(&valid), valid);
        let mut changed = valid.fields().clone();
        changed.activity = Activity::Working;
        assert_eq!(
            AgentSnapshot::new(changed),
            Err(ProtocolError::InvalidState)
        );
        let mut changed = valid.fields().clone();
        changed.interaction = Some(interaction(target(), kind));
        assert_eq!(
            AgentSnapshot::new(changed),
            Err(ProtocolError::TargetMismatch)
        );
    }
    let mut changed = fields();
    changed.capabilities.respond_interaction = Capability::Available {};
    assert_eq!(
        AgentSnapshot::new(changed),
        Err(ProtocolError::InvalidState)
    );
    let mut observed_only = waiting(InteractionKind::Approval).fields().clone();
    observed_only.capabilities.respond_interaction = Capability::Unsupported {};
    assert!(AgentSnapshot::new(observed_only).is_ok());
}

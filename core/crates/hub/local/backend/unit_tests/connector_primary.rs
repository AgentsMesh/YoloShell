use super::fixtures::{agent_control::host_view, target, view};
use crate::{connector_primary::primary, connectors::epoch_millis};
use std::{
    collections::BTreeMap,
    time::{Duration, UNIX_EPOCH},
};
use yolo_agent_contract::AgentState;
use yolo_hub_connector_registry::Catalogue;
use yolo_hub_identity::{AgentAddress, AgentId, AgentTarget, ExecutionId};
use yolo_runtime::model::RecoveryState;

const TIME_MILLIS: u64 = 123_456;
const BEFORE_EPOCH: Duration = Duration::from_millis(1);

#[test]
fn actual_native_binding_wins_and_connector_primary_requires_the_exact_live_epoch() {
    let terminal = target();
    let native = AgentAddress {
        hub_id: terminal.address.hub_id,
        agent_id: AgentId::new(),
    };
    let native = AgentTarget {
        address: native,
        execution_id: ExecutionId::new(),
    };
    let connector = AgentTarget {
        address: AgentAddress {
            agent_id: AgentId::new(),
            ..native.address
        },
        execution_id: ExecutionId::new(),
    };
    let catalogue = Catalogue {
        agents: vec![],
        primaries: [(terminal, connector)].into_iter().collect(),
        enabled_targets: Vec::new(),
    };
    assert_eq!(
        primary(
            terminal.address.hub_id,
            &host_view(),
            Some(native),
            &catalogue
        ),
        Some(native)
    );
    assert_eq!(
        primary(terminal.address.hub_id, &host_view(), None, &catalogue),
        None
    );
    assert_eq!(
        primary(terminal.address.hub_id, &view(), None, &catalogue),
        Some(connector)
    );
    let mut historical = host_view();
    historical.agent.as_mut().unwrap().state = AgentState::Exited;
    assert_eq!(
        primary(
            terminal.address.hub_id,
            &historical,
            Some(native),
            &catalogue
        ),
        Some(connector)
    );
    let mut changed = view();
    changed.runtime.as_mut().unwrap().runtime_epoch = uuid::Uuid::new_v4();
    assert_eq!(
        primary(terminal.address.hub_id, &changed, None, &catalogue),
        None
    );
    let mut uncertain = view();
    uncertain.manifest.recovery_state = RecoveryState::Uncertain;
    assert_eq!(
        primary(terminal.address.hub_id, &uncertain, None, &catalogue),
        None
    );
    let empty = Catalogue {
        agents: vec![],
        primaries: BTreeMap::new(),
        enabled_targets: Vec::new(),
    };
    assert_eq!(
        primary(terminal.address.hub_id, &historical, Some(native), &empty),
        Some(native)
    );
}

#[test]
fn publication_timestamp_is_server_epoch_time_and_rejects_pre_epoch_clock() {
    assert_eq!(
        epoch_millis(UNIX_EPOCH + Duration::from_millis(TIME_MILLIS)).unwrap(),
        TIME_MILLIS
    );
    assert!(epoch_millis(UNIX_EPOCH - BEFORE_EPOCH).is_err());
}

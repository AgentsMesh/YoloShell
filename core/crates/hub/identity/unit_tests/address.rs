use crate::constants::{AGENT_URI, ID, NIL, REMOVED_TERMINAL_ID_FIELD, TERMINAL_URI};
use serde_json::json;
use yolo_hub_identity::*;

#[test]
fn stable_addresses_do_not_contain_routes_or_runtime_generations() {
    let hub_id = ID.parse().unwrap();
    let session = TerminalAddress {
        hub_id,
        terminal_id: ID.parse().unwrap(),
    };
    let agent = AgentAddress {
        hub_id,
        agent_id: ID.parse().unwrap(),
    };
    assert_eq!(session.to_string(), TERMINAL_URI);
    assert_eq!(agent.to_string(), AGENT_URI);
    assert_eq!(
        serde_json::to_value(session).unwrap(),
        json!({(field::HUB_ID): ID, (field::TERMINAL_ID): ID})
    );
    assert_eq!(
        serde_json::to_value(agent).unwrap(),
        json!({(field::HUB_ID): ID, (field::AGENT_ID): ID})
    );
    assert_eq!(
        serde_json::from_value::<TerminalAddress>(
            json!({(field::HUB_ID): ID, (field::TERMINAL_ID): ID})
        )
        .unwrap(),
        session
    );
    assert_eq!(
        serde_json::from_value::<AgentAddress>(json!({(field::HUB_ID): ID, (field::AGENT_ID): ID}))
            .unwrap(),
        agent
    );
    let runtime = RuntimeTarget {
        address: session,
        runtime_epoch: RuntimeEpoch::new(),
    };
    let successor = RuntimeTarget {
        runtime_epoch: RuntimeEpoch::new(),
        ..runtime
    };
    assert_eq!(runtime.address, successor.address);
    assert_ne!(runtime, successor);
    let execution = AgentTarget {
        address: agent,
        execution_id: ExecutionId::new(),
    };
    let resumed = AgentTarget {
        execution_id: ExecutionId::new(),
        ..execution
    };
    assert_eq!(execution.address, resumed.address);
    assert_ne!(execution, resumed);
    assert_eq!(
        serde_json::from_value::<RuntimeTarget>(serde_json::to_value(runtime).unwrap()).unwrap(),
        runtime
    );
    assert_eq!(
        serde_json::from_value::<AgentTarget>(serde_json::to_value(execution).unwrap()).unwrap(),
        execution
    );
}

#[test]
fn addresses_reject_unbound_nil_identities_and_unknown_routing_fields() {
    for value in [
        json!({}),
        json!({(field::HUB_ID): ID}),
        json!({(field::HUB_ID): NIL, (field::TERMINAL_ID): ID}),
        json!({(field::HUB_ID): ID, (field::TERMINAL_ID): NIL}),
        json!({(field::HUB_ID): ID, (field::TERMINAL_ID): ID, "via": ID}),
    ] {
        assert!(serde_json::from_value::<TerminalAddress>(value).is_err());
    }
    for value in [
        json!({}),
        json!({(field::HUB_ID): ID}),
        json!({(field::HUB_ID): NIL, (field::AGENT_ID): ID}),
        json!({(field::HUB_ID): ID, (field::AGENT_ID): NIL}),
        json!({(field::HUB_ID): ID, (field::AGENT_ID): ID, "via": ID}),
    ] {
        assert!(serde_json::from_value::<AgentAddress>(value).is_err());
    }
}

#[test]
fn terminal_address_rejects_retired_field_and_ambiguous_double_identity() {
    for value in [
        json!({(field::HUB_ID): ID, (REMOVED_TERMINAL_ID_FIELD): ID}),
        json!({(field::HUB_ID): ID, (field::TERMINAL_ID): ID, (REMOVED_TERMINAL_ID_FIELD): ID}),
    ] {
        assert!(serde_json::from_value::<TerminalAddress>(value).is_err());
    }
}

#[test]
fn runtime_index_retains_different_epochs_of_the_same_session() {
    let address = TerminalAddress {
        hub_id: HubId::new(),
        terminal_id: TerminalId::new(),
    };
    let first = RuntimeTarget {
        address,
        runtime_epoch: RuntimeEpoch::new(),
    };
    let second = RuntimeTarget {
        address,
        runtime_epoch: RuntimeEpoch::new(),
    };
    let values = std::collections::BTreeMap::from([
        (first, first.runtime_epoch),
        (second, second.runtime_epoch),
    ]);
    assert_eq!(values.get(&first), Some(&first.runtime_epoch));
    assert_eq!(values.get(&second), Some(&second.runtime_epoch));
    assert_ne!(first.cmp(&second), std::cmp::Ordering::Equal);
    assert_eq!(first.partial_cmp(&second), Some(first.cmp(&second)));
}

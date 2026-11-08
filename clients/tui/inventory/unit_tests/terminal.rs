use crate::fixtures::Fixture;
use yolo_hub_agent_protocol::{AgentSnapshot, Freshness, Reachability};
use yolo_hub_identity::{
    AgentAddress, AgentId, AgentTarget, ExecutionId, HubId, RuntimeEpoch, TerminalId,
};
use yolo_hub_terminal_protocol::{RuntimeStatus, TerminalSummary, TerminalSummaryFields};

#[test]
fn terminal_lookup_requires_supported_current_reachable_observation_and_scope() {
    let fixture = Fixture::new();
    let terminals = fixture.terminals();
    let runtime_target = fixture.agent.fields().terminal;
    let address = fixture.agent.cursor().target().address;
    let agent_target = fixture.agent.cursor().target();
    let mut inventory = fixture.inventory();
    assert_eq!(
        inventory.terminal_for_selected(&terminals),
        Some(runtime_target)
    );
    inventory.current = false;
    assert_eq!(inventory.terminal_for_selected(&terminals), None);
    inventory.current = true;
    inventory.agents_supported = false;
    assert_eq!(inventory.terminal_for_selected(&terminals), None);
    inventory.agents_supported = true;
    inventory.selected_agent = None;
    assert_eq!(inventory.terminal_for_selected(&terminals), None);
    inventory.selected_agent = Some(yolo_hub_identity::AgentTarget {
        address: AgentAddress {
            agent_id: AgentId::new(),
            ..address
        },
        execution_id: yolo_hub_identity::ExecutionId::new(),
    });
    assert_eq!(inventory.terminal_for_selected(&terminals), None);
    inventory.selected_agent = Some(agent_target);
    inventory.scope = Some(HubId::new());
    assert_eq!(inventory.terminal_for_selected(&terminals), None);
    inventory.scope = Some(address.hub_id);
    assert_eq!(
        inventory.terminal_for_selected(&terminals),
        Some(runtime_target)
    );
    for stale in [true, false] {
        let mut fields = fixture.agent.fields().clone();
        if stale {
            fields.freshness = Freshness::Stale;
        } else {
            fields.reachability = Reachability::Unreachable;
        }
        inventory
            .agents
            .insert(agent_target, AgentSnapshot::new(fields).unwrap());
        assert_eq!(inventory.terminal_for_selected(&terminals), None);
    }
}

#[test]
fn an_agent_cannot_jump_into_another_binding_runtime_or_rekeyed_map_entry() {
    let fixture = Fixture::new();
    let target = fixture.agent.cursor().target();
    let mut inventory = fixture.inventory();
    let mutations: &[fn(&mut TerminalSummaryFields)] = &[
        |fields| fields.agent = None,
        |fields| {
            fields.agent = Some(AgentTarget {
                address: AgentAddress {
                    hub_id: fields.address.hub_id,
                    agent_id: AgentId::new(),
                },
                execution_id: ExecutionId::new(),
            })
        },
        |fields| fields.runtime = None,
        |fields| fields.runtime.as_mut().unwrap().target.runtime_epoch = RuntimeEpoch::new(),
        |fields| fields.archived = true,
    ];
    for mutation in mutations {
        let mut fields = fixture.session.fields().clone();
        mutation(&mut fields);
        let terminals = std::collections::BTreeMap::from([(
            fields.address,
            TerminalSummary::new(fields).unwrap(),
        )]);
        assert_eq!(inventory.terminal_for_selected(&terminals), None);
    }
    assert_eq!(
        inventory.terminal_for_selected(&std::collections::BTreeMap::new()),
        None
    );
    let mut fields = fixture.session.fields().clone();
    fields.address.terminal_id = TerminalId::new();
    fields.runtime.as_mut().unwrap().target.address = fields.address;
    let terminals = std::collections::BTreeMap::from([(
        fixture.session.fields().address,
        TerminalSummary::new(fields).unwrap(),
    )]);
    assert_eq!(inventory.terminal_for_selected(&terminals), None);
    let other = Fixture::new();
    inventory.agents.insert(target, other.agent);
    assert_eq!(inventory.terminal_for_selected(&fixture.terminals()), None);
}

#[test]
fn runtime_status_matrix_only_exposes_live_terminal_targets() {
    let fixture = Fixture::new();
    let inventory = fixture.inventory();
    for status in [
        RuntimeStatus::Starting,
        RuntimeStatus::Running,
        RuntimeStatus::Finalizing,
        RuntimeStatus::OutputBlocked,
        RuntimeStatus::Exited,
        RuntimeStatus::Lost,
        RuntimeStatus::Unreachable,
    ] {
        let mut fields = fixture.session.fields().clone();
        fields.runtime.as_mut().unwrap().status = status;
        let terminals = std::collections::BTreeMap::from([(
            fields.address,
            TerminalSummary::new(fields).unwrap(),
        )]);
        assert_eq!(
            inventory.terminal_for_selected(&terminals).is_some(),
            matches!(
                status,
                RuntimeStatus::Running | RuntimeStatus::OutputBlocked
            )
        );
    }
}

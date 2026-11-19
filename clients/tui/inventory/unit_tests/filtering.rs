use crate::{constants::*, fixtures::Fixture};
use yolo_hub_agent_protocol::{Activity, AgentSnapshot, Lifecycle};
use yolo_hub_terminal_protocol::{RuntimeStatus, TerminalSummary};
#[test]
fn scoped_search_matches_origin_provider_and_associated_session_metadata() {
    let first = Fixture::new();
    let second = Fixture::new();
    let mut terminals = first.terminals();
    terminals.extend(second.terminals());
    let mut inventory = first.inventory();
    inventory.replace(
        Some(vec![first.agent.clone(), second.agent.clone()]),
        Some(vec![first.hub.clone(), second.hub.clone()]),
    );
    for filter in [
        SESSION_NAME.to_lowercase(),
        AGENT_WORKING_DIRECTORY.into(),
        PROVIDER_KIND.into(),
        PROVIDER_VERSION.into(),
        NATIVE_REFERENCE.into(),
    ] {
        assert_eq!(inventory.visible_agents(&filter, &terminals).len(), 2);
    }
    for filter in [SESSION_NAME.to_uppercase(), WORKING_DIRECTORY.into()] {
        assert_eq!(inventory.visible_terminals(&filter, &terminals).len(), 2);
    }
    inventory.scope = Some(first.hub.hub_id);
    assert_eq!(
        inventory.visible_agents("", &terminals),
        vec![first.agent.cursor().target()]
    );
    assert_eq!(
        inventory.visible_terminals("", &terminals),
        vec![first.session.fields().address]
    );
    assert_eq!(inventory.visible_hubs("").len(), 2);
    assert_eq!(
        inventory.visible_hubs(&second.hub.hub_id.to_string()),
        vec![second.hub.hub_id]
    );
    assert!(
        inventory
            .visible_agents(NONMATCHING_FILTER, &terminals)
            .is_empty()
    );
    assert!(
        inventory
            .visible_terminals(NONMATCHING_FILTER, &terminals)
            .is_empty()
    );
    assert!(inventory.visible_hubs(NONMATCHING_FILTER).is_empty());
    assert_eq!(
        inventory
            .visible_agents(
                &first.agent.cursor().target().address.to_string(),
                &terminals
            )
            .len(),
        1
    );
    assert_eq!(
        inventory
            .visible_agents(&first.session.fields().address.to_string(), &terminals)
            .len(),
        1
    );
    assert_eq!(
        inventory
            .visible_terminals(&first.session.fields().address.to_string(), &terminals)
            .len(),
        1
    );
}
#[test]
fn search_uses_reported_states_and_never_guesses_provider_activity() {
    let fixture = Fixture::new();
    let terminals = fixture.terminals();
    let mut inventory = fixture.inventory();
    for (lifecycle, text) in [
        (Lifecycle::Unknown {}, "unknown"),
        (Lifecycle::Starting {}, "starting"),
        (Lifecycle::Running {}, "running"),
        (Lifecycle::Exited { exit_code: None }, "exited"),
    ] {
        let mut fields = fixture.agent.fields().clone();
        fields.lifecycle = lifecycle;
        fields.activity = Activity::Unknown;
        inventory.replace(
            Some(vec![AgentSnapshot::new(fields).unwrap()]),
            Some(vec![fixture.hub.clone()]),
        );
        let expected = matches!(lifecycle, Lifecycle::Exited { .. });
        if expected {
            assert!(inventory.visible_agents(text, &terminals).is_empty());
            inventory.history = true;
        }
        assert_eq!(inventory.visible_agents(text, &terminals).len(), 1);
        inventory.history = false;
    }
    for (activity, text) in [
        (Activity::Unknown, "unknown"),
        (Activity::Idle, "idle"),
        (Activity::Working, "working"),
        (Activity::WaitingForInput, "waiting for input"),
        (Activity::WaitingForApproval, "waiting for approval"),
    ] {
        let mut fields = fixture.agent.fields().clone();
        fields.activity = activity;
        inventory.replace(Some(vec![AgentSnapshot::new(fields).unwrap()]), None);
        assert_eq!(inventory.visible_agents(text, &terminals).len(), 1);
    }
}
#[test]
fn terminal_status_search_preserves_exit_and_archival_distinctions() {
    let fixture = Fixture::new();
    let mut inventory = fixture.inventory();
    for (status, text) in [
        (RuntimeStatus::Starting, "starting"),
        (RuntimeStatus::Running, "running"),
        (RuntimeStatus::Finalizing, "finalizing"),
        (RuntimeStatus::OutputBlocked, "output blocked"),
        (RuntimeStatus::Exited, "ended"),
        (RuntimeStatus::Lost, "lost"),
        (RuntimeStatus::Unreachable, "unreachable"),
    ] {
        let mut fields = fixture.session.fields().clone();
        fields.runtime.as_mut().unwrap().status = status;
        let terminals = std::collections::BTreeMap::from([(
            fields.address,
            TerminalSummary::new(fields).unwrap(),
        )]);
        let history = matches!(status, RuntimeStatus::Exited | RuntimeStatus::Lost);
        if history {
            assert!(inventory.visible_terminals(text, &terminals).is_empty());
            inventory.history = true;
        }
        assert_eq!(inventory.visible_terminals(text, &terminals).len(), 1);
        inventory.history = false;
    }
    let mut fields = fixture.session.fields().clone();
    fields.runtime = None;
    let mut terminals = std::collections::BTreeMap::from([(
        fields.address,
        TerminalSummary::new(fields.clone()).unwrap(),
    )]);
    assert_eq!(inventory.visible_terminals("unknown", &terminals).len(), 1);
    fields.archived = true;
    terminals.insert(fields.address, TerminalSummary::new(fields).unwrap());
    assert!(
        inventory
            .visible_terminals("archived", &terminals)
            .is_empty()
    );
    inventory.history = true;
    assert_eq!(inventory.visible_terminals("archived", &terminals).len(), 1);
}
#[test]
fn absent_associated_session_and_unknown_provider_metadata_are_searchable_by_origin() {
    let fixture = Fixture::new();
    let mut fields = fixture.agent.fields().clone();
    fields.provider =
        yolo_hub_agent_protocol::ObservedProvider::new(PROVIDER_KIND.into(), None, None).unwrap();
    let mut inventory = fixture.inventory();
    inventory.replace(Some(vec![AgentSnapshot::new(fields).unwrap()]), None);
    assert!(
        inventory
            .visible_agents(PROVIDER_KIND, &std::collections::BTreeMap::new())
            .is_empty()
    );
    inventory.history = true;
    assert_eq!(
        inventory
            .visible_agents(PROVIDER_KIND, &std::collections::BTreeMap::new())
            .len(),
        1
    );
    assert!(inventory.visible_hubs("").is_empty());
    inventory.agents_supported = false;
    assert!(
        inventory
            .visible_agents("", &fixture.terminals())
            .is_empty()
    );
}
#[test]
fn active_agent_rows_require_the_exact_terminal_binding() {
    let fixture = Fixture::new();
    let mut fields = fixture.session.fields().clone();
    fields.agent = None;
    let terminal = TerminalSummary::new(fields).unwrap();
    let terminals = std::collections::BTreeMap::from([(terminal.fields().address, terminal)]);
    let inventory = fixture.inventory();
    assert!(inventory.visible_agents("", &terminals).is_empty());
    let mut history = fixture.inventory();
    history.history = true;
    assert_eq!(history.visible_agents("", &terminals).len(), 1);
}

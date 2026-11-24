use crate::*;
use std::collections::BTreeMap;
use uuid::Uuid;
use yolo_hub_identity::{AgentRunId, HubId, TerminalId};

fn summary() -> AgentRunSummary {
    let hub = HubId::new();
    AgentRunSummary {
        address: AgentRunAddress {
            hub_id: hub,
            run_id: AgentRunId::new(),
        },
        revision: FIRST_AGENT_RUN_REVISION,
        name: "Alpha".into(),
        aliases: vec!["beta".into()],
        labels: BTreeMap::new(),
        provider_id: "codex".into(),
        workspace: None,
        parent: Some(AgentRunAddress {
            hub_id: hub,
            run_id: AgentRunId::new(),
        }),
        terminal: Some(yolo_hub_identity::TerminalAddress {
            hub_id: hub,
            terminal_id: TerminalId::new(),
        }),
        state: AgentRunState::Launched,
        current: None,
        start_token: Uuid::new_v4(),
        created_at_ms: 0,
        updated_at_ms: 0,
    }
}

#[test]
fn filters_match_public_summary_fields() {
    let mut run = summary();
    run.labels.insert("team".into(), "alpha".into());
    assert!(AgentRunFilter::Name { value: "ALPHA".into() }.matches(&run));
    assert!(AgentRunFilter::Name { value: "Beta".into() }.matches(&run));
    assert!(AgentRunFilter::State { is: vec![AgentRunState::Launched] }.matches(&run));
    assert!(!AgentRunFilter::State { is: vec![AgentRunState::Stopped] }.matches(&run));
    assert!(AgentRunFilter::Provider { id: "CODEX".into() }.matches(&run));
    assert!(!AgentRunFilter::Provider { id: "other".into() }.matches(&run));
    assert!(AgentRunFilter::Parent { address: run.parent.unwrap() }.matches(&run));
    assert!(AgentRunFilter::Terminal { address: run.terminal.unwrap() }.matches(&run));
    assert!(AgentRunFilter::Label { key: "team".into(), value: "alpha".into() }.matches(&run));
    assert!(!AgentRunFilter::Label { key: "team".into(), value: "beta".into() }.matches(&run));
}

use crate::fixtures::constants::*;
use crate::fixtures::{hub, id};

const AGENT_RUN_PAGE_FIXTURE: u16 = 16;

pub fn agent_run_summary() -> yolo_hub_agent_run_protocol::AgentRunSummary {
    yolo_hub_agent_run_protocol::AgentRunSummary {
        address: yolo_hub_agent_run_protocol::AgentRunAddress {
            hub_id: hub(HUB_ID),
            run_id: yolo_hub_identity::AgentRunId::from_uuid(id(AGENT_ID)).unwrap(),
        },
        revision: 1,
        name: "alpha".into(),
        aliases: Vec::new(),
        labels: Default::default(),
        provider_id: PROVIDER_KIND.into(),
        workspace: None,
        parent: None,
        terminal: Some(crate::fixtures::address(HUB_ID, SESSION_ID)),
        state: yolo_hub_agent_run_protocol::AgentRunState::Launched,
        current: None,
        start_token: id(REQUEST_ID),
        created_at_ms: 0,
        updated_at_ms: 0,
    }
}

pub fn get_agent_run() -> yolo_hub_agent_run_protocol::GetAgentRunRequest {
    yolo_hub_agent_run_protocol::GetAgentRunRequest::new(
        yolo_hub_agent_run_protocol::GetAgentRunFields {
            schema_version: yolo_hub_agent_run_protocol::AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
            selector: yolo_hub_agent_run_protocol::AgentRunSelector::Address(
                agent_run_summary().address,
            ),
        },
    )
    .unwrap()
}

pub fn list_agent_runs() -> yolo_hub_agent_run_protocol::ListAgentRunsRequest {
    yolo_hub_agent_run_protocol::ListAgentRunsRequest::new(
        yolo_hub_agent_run_protocol::ListAgentRunsFields {
            schema_version: yolo_hub_agent_run_protocol::AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
            after: None,
            limit: AGENT_RUN_PAGE_FIXTURE,
            filter: None,
        },
    )
    .unwrap()
}

pub fn update_agent_run() -> yolo_hub_agent_run_protocol::UpdateAgentRunRequest {
    yolo_hub_agent_run_protocol::UpdateAgentRunRequest::new(
        yolo_hub_agent_run_protocol::UpdateAgentRunFields {
            schema_version: yolo_hub_agent_run_protocol::AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
            request_id: id(REQUEST_ID),
            address: agent_run_summary().address,
            expected_revision: 1,
            name: Some("alpha".into()),
            aliases: None,
            labels: None,
        },
    )
    .unwrap()
}

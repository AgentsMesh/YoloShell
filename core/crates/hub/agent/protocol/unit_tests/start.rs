use crate::fixtures::*;
use uuid::Uuid;
use yolo_hub_agent_protocol::*;
use yolo_hub_agent_run_protocol::{AgentRunAddress, AgentRunSelector};
use yolo_hub_identity::{AgentRunId, HubId};

fn fields() -> AgentStartFields {
    AgentStartFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        start_token: Uuid::new_v4(),
        provider_id: "codex".into(),
        name: Some("started".into()),
        cwd: "/workspace".into(),
        size: yolo_hub_terminal_protocol::TerminalSize::default(),
        workspace_id: None,
        parent: None,
    }
}

#[test]
fn start_requests_carry_independent_identity() {
    let request = AgentStartRequest::new(fields()).unwrap();
    assert_ne!(request.fields().request_id, request.fields().start_token);
    let wire = serde_json::to_vec(&request).unwrap();
    assert_eq!(
        serde_json::from_slice::<AgentStartRequest>(&wire).unwrap(),
        request
    );
}

#[test]
fn start_requests_reject_bad_schema_and_identity() {
    let mut invalid = fields();
    invalid.schema_version += 1;
    assert_eq!(
        AgentStartRequest::new(invalid),
        Err(ProtocolError::UnsupportedSchema)
    );
    for nil in [true, false] {
        let mut invalid = fields();
        if nil {
            invalid.request_id = Uuid::nil();
        } else {
            invalid.start_token = Uuid::nil();
        }
        assert_eq!(
            AgentStartRequest::new(invalid),
            Err(ProtocolError::InvalidRequest)
        );
    }
}

#[test]
fn start_requests_accept_a_parent_selector_and_reject_bad_names() {
    let mut valid = fields();
    valid.parent = Some(AgentRunSelector::Address(AgentRunAddress {
        hub_id: HubId::new(),
        run_id: AgentRunId::new(),
    }));
    let request = AgentStartRequest::new(valid).unwrap();
    assert!(request.fields().parent.is_some());

    let mut named = fields();
    named.parent = Some(AgentRunSelector::Name("parent-run".into()));
    assert!(AgentStartRequest::new(named).is_ok());

    let mut invalid = fields();
    invalid.parent = Some(AgentRunSelector::Name(String::new()));
    assert_eq!(
        AgentStartRequest::new(invalid),
        Err(ProtocolError::InvalidRequest)
    );
}

#[test]
fn start_requests_reject_unknown_providers_and_bad_metadata() {
    for provider in ["", "codex cli", "../codex", "codex;rm"] {
        let mut invalid = fields();
        invalid.provider_id = provider.into();
        assert_eq!(
            AgentStartRequest::new(invalid),
            Err(ProtocolError::InvalidProvider)
        );
    }
    let mut invalid = fields();
    invalid.cwd = String::new();
    assert_eq!(
        AgentStartRequest::new(invalid),
        Err(ProtocolError::InvalidRequest)
    );
    let mut invalid = fields();
    invalid.name = Some("bad\0name".into());
    assert_eq!(
        AgentStartRequest::new(invalid),
        Err(ProtocolError::InvalidRequest)
    );
}

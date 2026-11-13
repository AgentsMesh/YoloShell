use crate::fixtures::agent;
use yolo_hub_agent_protocol::{Activity, AgentWaitCondition, AgentWaitRequest, AgentWaitResult};

pub fn request() -> AgentWaitRequest {
    AgentWaitRequest {
        targets: vec![agent().cursor().target()],
        runs: Vec::new(),
        condition: AgentWaitCondition::Activity {
            is: vec![Activity::WaitingForInput],
        },
        after: None,
        wait_ms: 100,
    }
}

pub fn result() -> AgentWaitResult {
    AgentWaitResult {
        targets: vec![agent().cursor().target()],
        matched: vec![agent()],
        ended: vec![],
        timed_out: false,
    }
}

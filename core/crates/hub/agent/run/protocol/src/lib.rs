//! Durable agent-run identity: a nameable orchestration object whose current
//! execution and terminal are separate, observed facts.

mod constants;
mod request;
mod run;

pub use constants::*;
pub use request::{
    AgentRunResult, GetAgentRunFields, GetAgentRunRequest, ListAgentRunsFields,
    ListAgentRunsRequest, ListAgentRunsResult, UpdateAgentRunFields, UpdateAgentRunRequest,
};
pub use run::{
    AgentRunAddress, AgentRunFilter, AgentRunSelector, AgentRunState, AgentRunSummary,
    annotate_current,
};

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

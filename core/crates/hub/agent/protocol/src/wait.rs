//! Typed long-poll conditions over agent observations.
//!
//! A wait names exact execution targets, a condition over public snapshot
//! fields, and an optional cursor: with a cursor, a condition only counts
//! once the target has observed a state newer than that cursor. The reply
//! reports the matched snapshots or the executions that ended while waiting.

use crate::{
    Activity, AgentSnapshot, Capability, Freshness, Lifecycle, ObservationCursor, ProtocolError,
    Reachability,
};
use serde::{Deserialize, Serialize};
use yolo_hub_identity::AgentTarget;

pub const MAX_AGENT_WAIT_MS: u32 = 1_000;
pub const MAX_AGENT_WAIT_TARGETS: usize = 64;
pub const MAX_AGENT_WAIT_FILTERS: usize = 32;
pub const MAX_AGENT_WAIT_DEPTH: usize = 4;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWaitRequest {
    pub targets: Vec<AgentTarget>,
    /// Durable runs whose current executions are resolved by the Hub before
    /// waiting. Each selector is expanded once; they are not retained.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runs: Vec<yolo_hub_agent_run_protocol::AgentRunSelector>,
    pub condition: AgentWaitCondition,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<ObservationCursor>,
    pub wait_ms: u32,
}

impl AgentWaitRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        let total = self.targets.len() + self.runs.len();
        if total == 0 || total > MAX_AGENT_WAIT_TARGETS || self.wait_ms == 0 {
            return Err(ProtocolError::InvalidRequest);
        }
        if self.wait_ms > MAX_AGENT_WAIT_MS {
            return Err(ProtocolError::InvalidRequest);
        }
        let mut unique = self.targets.clone();
        unique.sort();
        unique.dedup();
        if unique.len() != self.targets.len() {
            return Err(ProtocolError::InvalidRequest);
        }
        for (index, run) in self.runs.iter().enumerate() {
            match run {
                yolo_hub_agent_run_protocol::AgentRunSelector::Address(address) => {
                    if address.run_id.as_uuid().is_nil() {
                        return Err(ProtocolError::InvalidRequest);
                    }
                }
                yolo_hub_agent_run_protocol::AgentRunSelector::Name(name) => {
                    yolo_hub_agent_run_protocol::validate_name(name)
                        .map_err(|_| ProtocolError::InvalidRequest)?;
                }
            }
            if self.runs[index + 1..].contains(run) {
                return Err(ProtocolError::InvalidRequest);
            }
        }
        let mut budget = MAX_AGENT_WAIT_FILTERS;
        self.condition.validate(0, &mut budget)
    }
}

/// A condition over public agent fields. Lists are any-of, `All`/`Any`/`Not`
/// compose them, and nothing else about the snapshot is addressable.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operator", rename_all = "snake_case", deny_unknown_fields)]
pub enum AgentWaitCondition {
    All { filters: Vec<AgentWaitCondition> },
    Any { filters: Vec<AgentWaitCondition> },
    Not { filter: Box<AgentWaitCondition> },
    Activity { is: Vec<Activity> },
    Lifecycle { is: Vec<AgentLifecycleState> },
    Reachability { is: Vec<Reachability> },
    Freshness { is: Vec<Freshness> },
    /// The witness-backed submit capability is available: safe to prompt.
    Promptable,
    /// The witness-backed interrupt capability is available.
    Interruptible,
    /// A typed response can be delivered to the pending interaction.
    Respondable,
}

impl AgentWaitCondition {
    fn validate(&self, depth: usize, budget: &mut usize) -> Result<(), ProtocolError> {
        if depth > MAX_AGENT_WAIT_DEPTH || *budget == 0 {
            return Err(ProtocolError::InvalidRequest);
        }
        *budget -= 1;
        match self {
            Self::All { filters } | Self::Any { filters } => {
                if filters.is_empty() {
                    return Err(ProtocolError::InvalidRequest);
                }
                for filter in filters {
                    filter.validate(depth + 1, budget)?;
                }
            }
            Self::Not { filter } => filter.validate(depth + 1, budget)?,
            Self::Activity { is } => {
                if is.is_empty() {
                    return Err(ProtocolError::InvalidRequest);
                }
            }
            Self::Lifecycle { is } => {
                if is.is_empty() {
                    return Err(ProtocolError::InvalidRequest);
                }
            }
            Self::Reachability { is } => {
                if is.is_empty() {
                    return Err(ProtocolError::InvalidRequest);
                }
            }
            Self::Freshness { is } => {
                if is.is_empty() {
                    return Err(ProtocolError::InvalidRequest);
                }
            }
            Self::Promptable | Self::Interruptible | Self::Respondable => {}
        }
        Ok(())
    }

    pub fn matches(&self, snapshot: &AgentSnapshot) -> bool {
        let fields = snapshot.fields();
        match self {
            Self::All { filters } => filters.iter().all(|filter| filter.matches(snapshot)),
            Self::Any { filters } => filters.iter().any(|filter| filter.matches(snapshot)),
            Self::Not { filter } => !filter.matches(snapshot),
            Self::Activity { is } => is.contains(&fields.activity),
            Self::Lifecycle { is } => is.iter().any(|state| state.matches(&fields.lifecycle)),
            Self::Reachability { is } => is.contains(&fields.reachability),
            Self::Freshness { is } => is.contains(&fields.freshness),
            Self::Promptable => fields.capabilities.submit_prompt == Capability::Available {},
            Self::Interruptible => fields.capabilities.interrupt == Capability::Available {},
            Self::Respondable => {
                fields.capabilities.respond_interaction == Capability::Available {}
            }
        }
    }
}

/// A lifecycle state without its exit code: waiting on "exited" must not
/// require the caller to guess the code.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentLifecycleState {
    Unknown,
    Starting,
    Running,
    Exited,
}

impl AgentLifecycleState {
    pub fn of(lifecycle: &Lifecycle) -> Self {
        match lifecycle {
            Lifecycle::Unknown {} => Self::Unknown,
            Lifecycle::Starting {} => Self::Starting,
            Lifecycle::Running {} => Self::Running,
            Lifecycle::Exited { .. } => Self::Exited,
        }
    }

    pub fn matches(self, lifecycle: &Lifecycle) -> bool {
        self == Self::of(lifecycle)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWaitResult {
    /// Exact targets the Hub evaluated after resolving run selectors. The
    /// reply echoes the request's targets plus any run-expanded executions,
    /// so clients correlate matched/ended against this set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<AgentTarget>,
    pub matched: Vec<AgentSnapshot>,
    pub ended: Vec<AgentTarget>,
    pub timed_out: bool,
}
